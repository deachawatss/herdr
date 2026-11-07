use super::*;
use crate::api::schema::{Method, PaneMoveDestination, SplitDirection};

fn two_workspaces() -> ClientShellSnapshot {
    let mut projected = snapshot();
    let mut second = projected.workspaces[0].clone();
    second.workspace_id = "ws_2".into();
    second.active_tab_id = "tab_2".into();
    second.number = 2;
    second.label = "gale".into();
    second.focused = false;
    projected.workspaces.push(second);
    let mut tab = projected.tabs[0].clone();
    tab.tab_id = "tab_2".into();
    tab.workspace_id = "ws_2".into();
    tab.focused = false;
    projected.tabs.push(tab);
    let mut pane = projected.panes[0].clone();
    pane.pane_id = "pane_2".into();
    pane.workspace_id = "ws_2".into();
    pane.tab_id = "tab_2".into();
    pane.focused = false;
    projected.panes.push(pane);
    projected
}

fn composed(projected: ClientShellSnapshot) -> ClientShellState {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(surface());
    state.compose(106, 24).expect("two workspaces");
    state
}

fn mouse(kind: MouseEventKind, column: u16, row: u16) -> RawInputEvent {
    RawInputEvent::Mouse(MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::empty(),
    })
}

fn workspace_row(state: &ClientShellState, workspace_id: &str) -> Rect {
    state
        .hits
        .workspaces
        .iter()
        .find(|hit| hit.workspace_id == workspace_id)
        .expect("workspace row")
        .rect
}

fn agent_row(state: &ClientShellState, pane_id: &str) -> Rect {
    state
        .hits
        .agents
        .iter()
        .find(|(_, id)| id == pane_id)
        .expect("agent row")
        .0
}

fn drag_row_to(
    state: &mut ClientShellState,
    source: Rect,
    column: u16,
    row: u16,
) -> Vec<ClientShellAction> {
    state.handle_raw_events(vec![mouse(
        MouseEventKind::Down(MouseButton::Left),
        source.x + 2,
        source.y,
    )]);
    state.handle_raw_events(vec![mouse(
        MouseEventKind::Drag(MouseButton::Left),
        source.x + 2,
        source.y + 1,
    )]);
    state.handle_raw_events(vec![mouse(
        MouseEventKind::Drag(MouseButton::Left),
        column,
        row,
    )]);
    state
        .handle_raw_events(vec![mouse(
            MouseEventKind::Up(MouseButton::Left),
            column,
            row,
        )])
        .actions
}

fn drag_second_workspace_to(
    state: &mut ClientShellState,
    column: u16,
    row: u16,
) -> Vec<ClientShellAction> {
    let source = workspace_row(state, "ws_2");
    drag_row_to(state, source, column, row)
}

fn with_agent_on_second_pane(mut projected: ClientShellSnapshot) -> ClientShellSnapshot {
    projected.agents = vec![ClientShellAgent {
        pane_id: "pane_2".into(),
        workspace_id: "ws_2".into(),
        tab_id: "tab_2".into(),
        name: Some("gale".into()),
        display_agent: None,
        agent: Some("claude".into()),
        title: None,
        terminal_title: None,
        terminal_title_stripped: None,
        agent_status: AgentStatus::Idle,
        state_change_seq: 1,
        state_labels: Vec::new(),
        tokens: Vec::new(),
        focused: false,
    }];
    projected
}

fn sends_method(actions: &[ClientShellAction], name: &str) -> bool {
    actions.iter().any(|action| {
        matches!(
            action,
            ClientShellAction::Endpoint { request, .. }
                if crate::api::api_method_name(&request.method) == name
        )
    })
}

fn moved_split(actions: &[ClientShellAction]) -> Option<SplitDirection> {
    let [ClientShellAction::Endpoint { request, .. }] = actions else {
        return None;
    };
    let Method::PaneMove(params) = &request.method else {
        return None;
    };
    let PaneMoveDestination::Tab {
        tab_id,
        target_pane_id,
        split,
        ..
    } = &params.destination
    else {
        return None;
    };
    (params.pane_id == "pane_2" && tab_id == "tab_1" && target_pane_id.as_deref() == Some("pane_1"))
        .then(|| split.clone())
}

fn sends_pane_move(actions: &[ClientShellAction]) -> bool {
    actions.iter().any(|action| {
        matches!(
            action,
            ClientShellAction::Endpoint { request, .. }
                if matches!(request.method, Method::PaneMove(_))
        )
    })
}

fn pane_rect(state: &ClientShellState) -> Rect {
    state
        .hits
        .panes
        .iter()
        .find(|hit| hit.pane_id == "pane_1")
        .expect("focused pane")
        .rect
}

#[test]
fn a_workspace_dropped_near_the_right_edge_of_a_pane_splits_it_right() {
    let mut state = composed(two_workspaces());
    let target = pane_rect(&state);

    let actions = drag_second_workspace_to(&mut state, target.right() - 1, target.y);

    assert_eq!(moved_split(&actions), Some(SplitDirection::Right));
    assert!(state.chrome_drag.is_none());
}

#[test]
fn a_workspace_dropped_near_the_bottom_edge_of_a_pane_splits_it_down() {
    let mut state = composed(two_workspaces());
    let target = pane_rect(&state);

    let actions = drag_second_workspace_to(&mut state, target.x, target.bottom() - 1);

    assert_eq!(moved_split(&actions), Some(SplitDirection::Down));
}

#[test]
fn a_drop_on_a_pane_sends_no_pane_move_when_the_server_does_not_accept_pane_move() {
    let mut state = composed(two_workspaces());
    state.set_endpoint_methods(Some(vec!["workspace.move".into(), "pane.focus".into()]));
    let target = pane_rect(&state);

    let actions = drag_second_workspace_to(&mut state, target.right() - 1, target.y);

    assert!(!sends_pane_move(&actions));
    assert!(state.chrome_drag.is_none());
}

#[test]
fn a_workspace_with_more_than_one_pane_in_its_active_tab_sends_no_pane_move() {
    let mut projected = two_workspaces();
    let mut extra = projected.panes[1].clone();
    extra.pane_id = "pane_3".into();
    projected.panes.push(extra);
    let mut state = composed(projected);
    let target = pane_rect(&state);

    let actions = drag_second_workspace_to(&mut state, target.right() - 1, target.y);

    assert!(!sends_pane_move(&actions));
}

#[test]
fn the_focused_workspace_dropped_on_its_own_pane_sends_no_pane_move() {
    let mut state = composed(two_workspaces());
    let source = workspace_row(&state, "ws_1");
    let target = pane_rect(&state);

    let actions = drag_row_to(&mut state, source, target.right() - 1, target.y);

    assert!(!sends_pane_move(&actions));
}

#[test]
fn a_drop_on_a_pane_still_reorders_workspaces_when_the_server_does_not_accept_pane_move() {
    let mut state = composed(two_workspaces());
    state.set_endpoint_methods(Some(vec!["workspace.move".into(), "pane.focus".into()]));
    let target = pane_rect(&state);

    let actions = drag_second_workspace_to(&mut state, target.right() - 1, target.y);

    assert!(sends_method(&actions, "workspace.move"));
}

#[test]
fn a_drop_on_an_open_popup_sends_no_pane_move() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(two_workspaces()));
    state.set_pane_surface(surface_with_popup());
    state.compose(106, 24).expect("popup over pane");
    let popup = state.hits.popup.as_ref().expect("popup hit").rect;

    let actions = drag_second_workspace_to(&mut state, popup.x + 1, popup.y + 1);

    assert!(!sends_pane_move(&actions));
}

#[test]
fn an_agent_row_dropped_near_the_right_edge_of_a_pane_splits_it_right() {
    let mut state = composed(with_agent_on_second_pane(two_workspaces()));
    let source = agent_row(&state, "pane_2");
    let target = pane_rect(&state);

    let actions = drag_row_to(&mut state, source, target.right() - 1, target.y);

    assert_eq!(moved_split(&actions), Some(SplitDirection::Right));
    assert!(state.chrome_drag.is_none());
}

#[test]
fn an_agent_row_moves_even_when_its_tab_has_other_panes() {
    let mut projected = with_agent_on_second_pane(two_workspaces());
    let mut extra = projected.panes[1].clone();
    extra.pane_id = "pane_3".into();
    projected.panes.push(extra);
    let mut state = composed(projected);
    let source = agent_row(&state, "pane_2");
    let target = pane_rect(&state);

    let actions = drag_row_to(&mut state, source, target.x, target.bottom() - 1);

    assert_eq!(moved_split(&actions), Some(SplitDirection::Down));
}

#[test]
fn a_click_on_an_agent_row_focuses_its_pane_on_release() {
    let mut state = composed(with_agent_on_second_pane(two_workspaces()));
    let source = agent_row(&state, "pane_2");

    let pressed = state.handle_raw_events(vec![mouse(
        MouseEventKind::Down(MouseButton::Left),
        source.x + 2,
        source.y,
    )]);
    let released = state.handle_raw_events(vec![mouse(
        MouseEventKind::Up(MouseButton::Left),
        source.x + 2,
        source.y,
    )]);

    assert!(pressed.actions.is_empty());
    assert!(matches!(
        &released.actions[..],
        [ClientShellAction::Endpoint { request, .. }]
            if matches!(&request.method, Method::PaneFocus(target) if target.pane_id == "pane_2")
    ));
}
