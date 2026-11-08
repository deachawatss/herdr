use super::*;

fn sidebar_text(prefixes: &[&str], label: &str) -> String {
    let mut config = Config::default();
    config.ui.sidebar.spaces.hide_label_prefixes =
        prefixes.iter().map(|prefix| prefix.to_string()).collect();
    let mut projected = snapshot();
    projected.workspaces[0].label = label.into();
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&config));
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(surface());
    let frame = state.compose(106, 20).expect("composed frame");
    frame
        .cells
        .chunks(frame.width as usize)
        .map(|row| {
            row.iter()
                .map(|cell| cell.symbol.as_str())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_hidden_prefix_is_cut_from_each_part_of_a_workspace_name() {
    let text = sidebar_text(&["NWFTH-"], "gale-oracle/NWFTH-PM");

    assert!(text.contains("gale-oracle/PM"), "{text}");
}

#[test]
fn a_name_that_is_only_the_prefix_keeps_it() {
    let text = sidebar_text(&["NWFTH-"], "NWFTH-");

    assert!(text.contains("NWFTH-"), "{text}");
}

#[test]
fn workspace_names_are_shown_whole_by_default() {
    let text = sidebar_text(&[], "NWFTH-ERP");

    assert!(text.contains("NWFTH-ERP"), "{text}");
}

#[test]
fn hide_label_prefixes_parses() {
    let config: Config =
        toml::from_str("[ui.sidebar.spaces]\nhide_label_prefixes = [\"NWFTH-\"]\n").unwrap();

    assert_eq!(config.ui.sidebar.spaces.hide_label_prefixes, vec!["NWFTH-"]);
}
