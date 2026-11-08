use super::*;
use crate::api::schema::{
    Method, PaneMoveDestination, PaneMoveParams, PaneRenameParams, WorkspaceRenameParams,
};

impl ClientShellState {
    pub(super) fn pane_name(&self, pane_id: &str) -> Option<String> {
        let snapshot = self.snapshot.as_deref()?;
        let pane = snapshot.panes.iter().find(|pane| pane.pane_id == pane_id)?;
        if let Some(label) = &pane.label {
            return Some(label.clone());
        }
        let shares_its_workspace = snapshot
            .panes
            .iter()
            .any(|other| other.workspace_id == pane.workspace_id && other.pane_id != pane.pane_id);
        if !shares_its_workspace {
            return snapshot
                .workspaces
                .iter()
                .find(|workspace| workspace.workspace_id == pane.workspace_id)
                .map(|workspace| workspace.label.clone());
        }
        pane.cwd.as_deref().and_then(|cwd| {
            std::path::Path::new(cwd)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
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
        let Some(name) = self.pane_name(&pane.pane_id) else {
            return Vec::new();
        };
        let target_workspace_id = match &params.destination {
            PaneMoveDestination::Tab { tab_id, .. } => snapshot
                .tabs
                .iter()
                .find(|tab| &tab.tab_id == tab_id)
                .map(|tab| tab.workspace_id.clone()),
            _ => None,
        };
        if target_workspace_id.as_ref() == Some(&pane.workspace_id) {
            return Vec::new();
        }
        let workspace_label = |workspace_id: &str| {
            snapshot
                .workspaces
                .iter()
                .find(|workspace| workspace.workspace_id == workspace_id)
                .map(|workspace| workspace.label.clone())
        };
        let mut renames = Vec::new();
        let source_keeps_panes = snapshot
            .panes
            .iter()
            .any(|other| other.workspace_id == pane.workspace_id && other.pane_id != pane.pane_id);
        if let Some(label) = workspace_label(&pane.workspace_id).filter(|_| source_keeps_panes) {
            let rest = without_segment(&label, &name);
            if !rest.is_empty() && rest != label {
                renames.push(Method::WorkspaceRename(WorkspaceRenameParams {
                    workspace_id: pane.workspace_id.clone(),
                    label: rest,
                }));
            }
        }
        let Some(target_workspace_id) = target_workspace_id else {
            return renames;
        };
        if let Some(target_label) = workspace_label(&target_workspace_id) {
            renames.push(Method::WorkspaceRename(WorkspaceRenameParams {
                workspace_id: target_workspace_id.clone(),
                label: format!("{target_label}/{name}"),
            }));
            let target_panes = snapshot
                .panes
                .iter()
                .filter(|other| other.workspace_id == target_workspace_id)
                .collect::<Vec<_>>();
            if let [only] = target_panes.as_slice() {
                if only.label.is_none() {
                    renames.push(Method::PaneRename(PaneRenameParams {
                        pane_id: only.pane_id.clone(),
                        label: Some(target_label),
                    }));
                }
            }
        }
        if pane.label.is_none() {
            renames.push(Method::PaneRename(PaneRenameParams {
                pane_id: pane.pane_id.clone(),
                label: Some(name),
            }));
        }
        renames
    }
}

fn without_segment(label: &str, name: &str) -> String {
    let mut segments = label.split('/').collect::<Vec<_>>();
    if let Some(index) = segments.iter().position(|segment| *segment == name) {
        segments.remove(index);
    }
    segments.join("/")
}
