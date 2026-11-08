use super::*;
use crate::api::schema::{Method, PaneMoveDestination};

fn state_with_panes(pane_count: usize) -> ClientShellState {
    let mut projected = snapshot();
    for number in 2..=pane_count {
        let mut pane = projected.panes[0].clone();
        pane.pane_id = format!("pane_{number}");
        pane.focused = false;
        pane.cwd = Some("/home/wind/ghq/github.com/deachawatss/gale-oracle".into());
        projected.panes.push(pane);
    }
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(surface());
    state.compose(106, 24).unwrap();
    state
}

fn pane_menu_labels(state: &mut ClientShellState, pane_id: &str) -> Vec<&'static str> {
    state.open_pane_context_menu(pane_id.into(), 10, 5);
    match state.overlay.as_ref() {
        Some(ClientShellOverlay::ContextMenu(menu)) => {
            menu.items().iter().map(|item| item.label).collect()
        }
        _ => panic!("pane menu"),
    }
}

#[test]
fn pane_menu_moves_a_pane_that_shares_its_workspace_to_a_new_workspace() {
    let mut state = state_with_panes(2);
    let labels = pane_menu_labels(&mut state, "pane_2");
    let index = labels
        .iter()
        .position(|label| *label == "Move to new workspace")
        .expect("the menu offers the move");

    let mut outcome = ClientShellInput::default();
    state.activate_context_menu_item(index, &mut outcome);

    let moved = outcome.actions.iter().find_map(|action| match action {
        ClientShellAction::Endpoint { request, .. } => match &request.method {
            Method::PaneMove(params) => match &params.destination {
                PaneMoveDestination::NewWorkspace { label, .. } => {
                    Some((params.pane_id.clone(), label.clone(), params.focus))
                }
                _ => None,
            },
            _ => None,
        },
        _ => None,
    });
    assert_eq!(
        moved,
        Some(("pane_2".to_string(), Some("gale-oracle".to_string()), true))
    );
}

#[test]
fn pane_menu_has_no_move_for_the_only_pane_of_a_workspace() {
    let mut state = state_with_panes(1);
    let labels = pane_menu_labels(&mut state, "pane_1");
    assert!(!labels.contains(&"Move to new workspace"), "{labels:?}");
}
