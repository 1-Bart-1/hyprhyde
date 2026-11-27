//! Hidden windows wait in `special:hyprhyde-<origin workspace id>`, so the workspace name is the
//! only record of where a window came from.

use crate::hyprland::{self, Client};
use std::io;

const PREFIX: &str = "special:hyprhyde-";

/// Name of the special workspace parking windows hidden from workspace `origin`.
pub fn parking_workspace(origin: i64) -> String {
    format!("{PREFIX}{origin}")
}

/// Origin workspace id of a parking workspace name, or `None` for any other workspace.
pub fn origin_workspace(name: &str) -> Option<i64> {
    name.strip_prefix(PREFIX)?.parse().ok()
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
    let parking = parking_workspace(window.workspace.id);
    Some(move_lua(&window.address, &format!("'{parking}'")))
}

/// Park the focused window; does nothing without one or on a special workspace.
pub fn hide_active() -> io::Result<()> {
    match hyprland::active_window()?.as_ref().and_then(hide_lua) {
        Some(expression) => hyprland::dispatch(&expression),
        None => Ok(()),
    }
}

/// Move the window at `address` onto `workspace` and focus it.
pub fn restore(address: &str, workspace: i64) -> io::Result<()> {
    hyprland::dispatch(&move_lua(address, &workspace.to_string()))?;
    let window = hyprland::window_lua(address);
    hyprland::dispatch(&format!("hl.dsp.focus({{ window = {window} }})"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hyprland::WorkspaceRef;

    fn window_on(id: i64, name: &str) -> Client {
        Client {
            address: "0x55ab".into(),
            class: "kitty".into(),
            title: "~".into(),
            workspace: WorkspaceRef { id, name: name.into() },
        }
    }

    #[test]
    fn origin_survives_the_round_trip_through_the_name() {
        for origin in [1, 4, 10, -1337] {
            assert_eq!(origin_workspace(&parking_workspace(origin)), Some(origin));
        }
        assert_eq!(parking_workspace(4), "special:hyprhyde-4");
    }

    #[test]
    fn other_workspaces_have_no_origin() {
        for name in
            ["4", "special:scratchpad", "special:hyprhyde-", "special:hyprhyde-x", "hyprhyde-4"]
        {
            assert_eq!(origin_workspace(name), None, "{name}");
        }
    }

    #[test]
    fn hiding_parks_the_window_under_its_workspace_id() {
        assert_eq!(
            hide_lua(&window_on(4, "4")).unwrap(),
            "hl.dsp.window.move({ workspace = 'special:hyprhyde-4', follow = false, \
             window = assert(hl.get_window([[address:0x55ab]])) })"
        );
    }

    #[test]
    fn hiding_on_a_special_workspace_does_nothing() {
        assert_eq!(hide_lua(&window_on(-98, "special:scratchpad")), None);
        assert_eq!(hide_lua(&window_on(-97, "special:hyprhyde-4")), None);
    }

    #[test]
    fn restoring_moves_onto_a_numeric_workspace() {
        assert_eq!(
            move_lua("0x55ab", "4"),
            "hl.dsp.window.move({ workspace = 4, follow = false, \
             window = assert(hl.get_window([[address:0x55ab]])) })"
        );
    }
}
