use super::*;
use crate::api::schema::{Method, PaneMoveDestination, PaneMoveParams, WorkspaceRenameParams};

impl ClientShellState {
    pub(super) fn pane_shares_its_workspace(&self, pane_id: &str) -> bool {
        let Some(snapshot) = self.snapshot.as_deref() else {
            return false;
        };
        let Some(pane) = snapshot.panes.iter().find(|pane| pane.pane_id == pane_id) else {
            return false;
        };
        snapshot
            .panes
            .iter()
            .any(|other| other.workspace_id == pane.workspace_id && other.pane_id != pane_id)
    }

    pub(super) fn pane_name(&self, pane_id: &str) -> Option<String> {
        let snapshot = self.snapshot.as_deref()?;
        let pane = snapshot.panes.iter().find(|pane| pane.pane_id == pane_id)?;
        let workspace_label = &snapshot
            .workspaces
            .iter()
            .find(|workspace| workspace.workspace_id == pane.workspace_id)?
            .label;
        if !self.pane_shares_its_workspace(pane_id) {
            return Some(workspace_label.clone());
        }
        let folder = folder_name(pane.cwd.as_deref())?;
        Some(
            workspace_label
                .split('/')
                .find(|segment| segment.eq_ignore_ascii_case(&folder))
                .map_or(folder.clone(), str::to_owned),
        )
    }

    pub(super) fn push_pane_move(&mut self, method: Method, outcome: &mut ClientShellInput) {
        let renames = match &method {
            Method::PaneMove(params) => self.renames_for_pane_move(params),
            _ => Vec::new(),
        };
        self.push_endpoint_method(method, outcome);
        for rename in renames {
            if self.supports_endpoint_method(&rename) {
                self.push_endpoint_method(rename, outcome);
            }
        }
    }

    fn renames_for_pane_move(&self, params: &PaneMoveParams) -> Vec<Method> {
        let Some(snapshot) = self.snapshot.as_deref() else {
            return Vec::new();
        };
        let Some(pane) = snapshot
            .panes
            .iter()
            .find(|pane| pane.pane_id == params.pane_id)
        else {
            return Vec::new();
        };
        let zoomed = |tab_id: &str| {
            snapshot
                .tabs
                .iter()
                .any(|tab| tab.tab_id == tab_id && tab.zoomed)
        };
        let target_tab_id = match &params.destination {
            PaneMoveDestination::Tab { tab_id, .. } => Some(tab_id.as_str()),
            _ => None,
        };
        if zoomed(&pane.tab_id) || target_tab_id.is_some_and(zoomed) {
            return Vec::new();
        }
        let target_workspace = target_tab_id
            .and_then(|tab_id| snapshot.tabs.iter().find(|tab| tab.tab_id == tab_id))
            .and_then(|tab| {
                snapshot
                    .workspaces
                    .iter()
                    .find(|workspace| workspace.workspace_id == tab.workspace_id)
            });
        if target_workspace.is_some_and(|workspace| workspace.workspace_id == pane.workspace_id) {
            return Vec::new();
        }
        let Some(name) = self.pane_name(&pane.pane_id) else {
            return Vec::new();
        };
        let mut renames = Vec::new();
        if self.pane_shares_its_workspace(&pane.pane_id) {
            let source_label = snapshot
                .workspaces
                .iter()
                .find(|workspace| workspace.workspace_id == pane.workspace_id)
                .map(|workspace| workspace.label.as_str())
                .unwrap_or_default();
            let mut segments = source_label.split('/').collect::<Vec<_>>();
            if let Some(index) = segments.iter().position(|segment| *segment == name) {
                segments.remove(index);
                if !segments.is_empty() {
                    renames.push(Method::WorkspaceRename(WorkspaceRenameParams {
                        workspace_id: pane.workspace_id.clone(),
                        label: segments.join("/"),
                    }));
                }
            }
        }
        if let Some(target) = target_workspace {
            renames.push(Method::WorkspaceRename(WorkspaceRenameParams {
                workspace_id: target.workspace_id.clone(),
                label: format!("{}/{name}", target.label),
            }));
        }
        renames
    }
}

fn folder_name(cwd: Option<&str>) -> Option<String> {
    std::path::Path::new(cwd?)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
}
