//! Hyprland IPC over its UNIX sockets: requests, JSON queries, Lua dispatches and event lines.

use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

/// A workspace as Hyprland reports it inside other objects.
#[derive(Deserialize)]
pub struct WorkspaceRef {
    /// Numeric id; Hyprland leaves it out for special workspaces.
    pub id: Option<i64>,
    pub name: String,
}

/// Reply to `j/activeworkspace`: the active workspace of the focused monitor.
#[derive(Deserialize)]
pub struct ActiveWorkspace {
    pub id: i64,
    pub monitor: String,
}

/// One entry of `j/clients`, or the reply to `j/activewindow`.
#[derive(Deserialize)]
pub struct Client {
    pub address: String,
    pub class: String,
    pub title: String,
    pub workspace: WorkspaceRef,
}

/// Directory holding this Hyprland instance's sockets.
fn socket_dir() -> io::Result<PathBuf> {
    let variable = |name: &str| {
        std::env::var(name).map_err(|_| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("{name} is not set; is Hyprland running?"),
            )
        })
    };
    Ok(PathBuf::from(variable("XDG_RUNTIME_DIR")?)
        .join("hypr")
        .join(variable("HYPRLAND_INSTANCE_SIGNATURE")?))
}

/// Path of the event socket (`.socket2.sock`).
pub fn event_socket() -> io::Result<PathBuf> {
    Ok(socket_dir()?.join(".socket2.sock"))
}

/// Send one request to `.socket.sock` and return the full reply.
fn request(command: &str) -> io::Result<String> {
    let mut stream = UnixStream::connect(socket_dir()?.join(".socket.sock"))?;
    stream.write_all(command.as_bytes())?;
    stream.shutdown(std::net::Shutdown::Write)?;
    let mut reply = String::new();
    stream.read_to_string(&mut reply)?;
    Ok(reply)
}

/// Run a JSON query such as `j/clients` and decode the reply.
pub fn query<T: DeserializeOwned>(command: &str) -> io::Result<T> {
    let reply = request(command)?;
    serde_json::from_str(&reply).map_err(|error| {
        io::Error::new(io::ErrorKind::InvalidData, format!("{command}: {error}: {reply}"))
    })
}

/// The focused window, or `None` when nothing is focused.
pub fn active_window() -> io::Result<Option<Client>> {
    decode_active_window(query("j/activewindow")?)
}

/// Decode a `j/activewindow` reply, where `{}` means nothing is focused.
fn decode_active_window(reply: serde_json::Value) -> io::Result<Option<Client>> {
    if reply.as_object().is_some_and(serde_json::Map::is_empty) {
        return Ok(None);
    }
    serde_json::from_value(reply).map(Some).map_err(|error| {
        io::Error::new(io::ErrorKind::InvalidData, format!("j/activewindow: {error}"))
    })
}

/// Run a Lua dispatcher expression such as `hl.dsp.focus({ ... })`.
pub fn dispatch(expression: &str) -> io::Result<()> {
    let reply = request(&format!("dispatch {expression}"))?;
    if reply.trim() == "ok" {
        Ok(())
    } else {
        Err(io::Error::other(format!("dispatch {expression}: {}", reply.trim())))
    }
}

/// Lua expression selecting the window at `address`, failing the dispatch if it is gone.
pub fn window_lua(address: &str) -> String {
    format!("assert(hl.get_window([[address:{address}]]))")
}

/// Name of an event line `name>>data`, or `None` for a malformed line.
pub fn event_name(line: &str) -> Option<&str> {
    line.split_once(">>").map(|(name, _)| name).filter(|name| !name.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_name_is_the_part_before_the_separator() {
        assert_eq!(event_name("workspacev2>>4,4"), Some("workspacev2"));
        assert_eq!(event_name("windowtitlev2>>55aa,a >> b"), Some("windowtitlev2"));
        assert_eq!(event_name("closewindow>>"), Some("closewindow"));
    }

    #[test]
    fn malformed_event_lines_have_no_name() {
        assert_eq!(event_name(""), None);
        assert_eq!(event_name("garbage"), None);
        assert_eq!(event_name(">>data"), None);
    }

    #[test]
    fn window_lua_selects_by_address() {
        assert_eq!(window_lua("0x55ab"), "assert(hl.get_window([[address:0x55ab]]))");
    }

    #[test]
    fn clients_decode_on_regular_and_special_workspaces() {
        // Hyprland 0.57 writes no `id` for a client on a special workspace.
        let json = r#"[{"address":"0x55ab","mapped":true,"class":"kitty","title":"~",
            "workspace":{"address":"special:hyprhyde-4","type":"special","name":"special:hyprhyde-4"}},
            {"address":"0x55ac","class":"firefox","title":"web",
            "workspace":{"address":"2","type":"normal","id":2,"name":"2"}}]"#;
        let clients: Vec<Client> = serde_json::from_str(json).unwrap();
        assert_eq!(clients[0].workspace.name, "special:hyprhyde-4");
        assert_eq!(clients[0].workspace.id, None);
        assert_eq!(clients[1].workspace.id, Some(2));
        assert_eq!(clients[1].class, "firefox");
    }

    #[test]
    fn active_window_is_none_only_when_nothing_is_focused() {
        assert!(decode_active_window(serde_json::json!({})).unwrap().is_none());
        let on_special = serde_json::json!({"address": "0x55ab", "class": "kitty", "title": "~",
            "workspace": {"address": "special:hyprhyde-4", "type": "special",
                          "name": "special:hyprhyde-4"}});
        assert!(decode_active_window(on_special).unwrap().is_some());
        assert!(decode_active_window(serde_json::json!({"address": "0x55ab"})).is_err());
    }
}
