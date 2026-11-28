//! Hidden windows wait in `special:hyprhyde-<origin workspace name>`, so the workspace name is the
//! only record of where a window came from.

use crate::hyprland::{self, Client};
use std::io;

const PREFIX: &str = "special:hyprhyde-";

/// Name of the special workspace parking windows hidden from the workspace named `origin`.
fn parking_workspace(origin: &str) -> String {
    format!("{PREFIX}{origin}")
}

/// Origin workspace name of a parking workspace name, or `None` for any other workspace.
pub fn origin_workspace(name: &str) -> Option<&str> {
    name.strip_prefix(PREFIX).filter(|origin| !origin.is_empty())
}

/// Lua value selecting the workspace named `name`: its number, or `name:` for a named one.
fn workspace_lua(name: &str) -> String {
    match name.parse::<i64>() {
        Ok(number) => number.to_string(),
        Err(_) => format!("[[name:{name}]]"),
    }
}

/// Lua dispatch moving the window at `address` to `workspace`, a Lua value, without following it.
fn move_lua(address: &str, workspace: &str) -> String {
    let window = hyprland::window_lua(address);
    format!("hl.dsp.window.move({{ workspace = {workspace}, follow = false, window = {window} }})")
}

/// Lua dispatch parking `window`, or `None` when it already sits on a special workspace.
fn hide_lua(window: &Client) -> Option<String> {
    if window.workspace.name.starts_with("special:") {
        return None;
    }
    let parking = parking_workspace(&window.workspace.name);
    Some(move_lua(&window.address, &format!("[[{parking}]]")))
}

/// Park the focused window; does nothing without one or on a special workspace.
pub fn hide_active() -> io::Result<()> {
    match hyprland::active_window()?.as_ref().and_then(hide_lua) {
        Some(expression) => hyprland::dispatch(&expression),
        None => Ok(()),
    }
}

/// Move the window at `address` onto the workspace named `workspace` and focus it.
pub fn restore(address: &str, workspace: &str) -> io::Result<()> {
    hyprland::dispatch(&move_lua(address, &workspace_lua(workspace)))?;
    let window = hyprland::window_lua(address);
    hyprland::dispatch(&format!("hl.dsp.focus({{ window = {window} }})"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hyprland::WorkspaceRef;

    fn window_on(name: &str) -> Client {
        Client {
            address: "0x55ab".into(),
            class: "kitty".into(),
            title: "~".into(),
            workspace: WorkspaceRef { name: name.into() },
        }
    }

    #[test]
    fn origin_survives_the_round_trip_through_the_name() {
        for origin in ["1", "4", "10", "foo", "my work"] {
            assert_eq!(origin_workspace(&parking_workspace(origin)), Some(origin));
        }
        assert_eq!(parking_workspace("4"), "special:hyprhyde-4");
    }

    #[test]
    fn other_workspaces_have_no_origin() {
        for name in ["4", "special:scratchpad", "special:hyprhyde-", "hyprhyde-4"] {
            assert_eq!(origin_workspace(name), None, "{name}");
        }
    }

    #[test]
    fn hiding_parks_the_window_under_its_workspace_name() {
        assert_eq!(
            hide_lua(&window_on("4")).unwrap(),
            "hl.dsp.window.move({ workspace = [[special:hyprhyde-4]], follow = false, \
             window = assert(hl.get_window([[address:0x55ab]])) })"
        );
        assert!(hide_lua(&window_on("foo")).unwrap().contains("[[special:hyprhyde-foo]]"));
    }

    #[test]
    fn hiding_on_a_special_workspace_does_nothing() {
        assert_eq!(hide_lua(&window_on("special:scratchpad")), None);
        assert_eq!(hide_lua(&window_on("special:hyprhyde-4")), None);
    }

    #[test]
    fn restoring_selects_numbered_workspaces_by_number_and_others_by_name() {
        assert_eq!(workspace_lua("4"), "4");
        assert_eq!(workspace_lua("foo"), "[[name:foo]]");
        assert_eq!(
            move_lua("0x55ab", &workspace_lua("4")),
            "hl.dsp.window.move({ workspace = 4, follow = false, \
             window = assert(hl.get_window([[address:0x55ab]])) })"
        );
    }
}
