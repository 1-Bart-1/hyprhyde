//! The bubble daemon: one small layer-shell window per hidden window of the active workspace.

use crate::hyprland::{self, ActiveWorkspace, Client};
use crate::{icon, parking};
use gtk::prelude::*;
use gtk::{gdk, gio, glib};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::str::FromStr;
use std::time::Duration;

const BUBBLE_SIZE: i32 = 44;
const BUBBLE_GAP: i32 = 8;
const SCREEN_MARGIN: i32 = 16;
const ANIMATION_MS: u32 = 180;

const CSS: &str = "
window.hyprhyde { background: none; }
.bubble {
    min-width: 44px; min-height: 44px; padding: 0; border-radius: 9999px;
    background: alpha(#1e1e2e, 0.85); background-image: none; box-shadow: none;
    border: 1px solid #45475a; transition: background-color 150ms ease, border-color 150ms ease;
}
.bubble:hover { background-color: alpha(#313244, 0.95); border-color: #89b4fa; }
";

/// Events after which the set of bubbles may have changed.
const REFRESH_EVENTS: [&str; 8] = [
    "workspacev2",
    "focusedmonv2",
    "movewindowv2",
    "closewindow",
    "windowtitlev2",
    "moveworkspacev2",
    "monitoradded",
    "monitorremoved",
];

/// Screen corner the bubbles stack from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Corner {
    vertical: Edge,
    horizontal: Edge,
}

impl FromStr for Corner {
    type Err = String;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        let (vertical, horizontal) = match name {
            "bottom-right" => (Edge::Bottom, Edge::Right),
            "bottom-left" => (Edge::Bottom, Edge::Left),
            "top-right" => (Edge::Top, Edge::Right),
            "top-left" => (Edge::Top, Edge::Left),
            _ => return Err(format!("unknown corner {name:?}")),
        };
        Ok(Self { vertical, horizontal })
    }
}

impl Default for Corner {
    fn default() -> Self {
        Self { vertical: Edge::Bottom, horizontal: Edge::Right }
    }
}

/// A hidden window as one bubble shows it.
#[derive(Clone)]
struct Hidden {
    address: String,
    class: String,
    title: String,
    workspace: i64,
    monitor: String,
}

/// The windows hidden from the focused monitor's active workspace, in the order Hyprland lists them.
fn hidden_on(active: &ActiveWorkspace, clients: Vec<Client>) -> Vec<Hidden> {
    clients
        .into_iter()
        .filter(|client| parking::origin_workspace(&client.workspace.name) == Some(active.id))
        .map(|client| Hidden {
            address: client.address,
            class: client.class,
            title: client.title,
            workspace: active.id,
            monitor: active.monitor.clone(),
        })
        .collect()
}

/// A bubble on screen and the window it stands for.
struct Bubble {
    hidden: Hidden,
    window: gtk::Window,
    revealer: gtk::Revealer,
}

/// The bubbles on screen, keyed by window address.
struct Bubbles {
    app: gtk::Application,
    corner: Corner,
    shown: HashMap<String, Bubble>,
}

impl Bubbles {
    /// Make the bubbles on screen match `hidden`, stacked from the corner in its order.
    fn show(&mut self, hidden: Vec<Hidden>) {
        let stale = self.shown.extract_if(|_, bubble| {
            !hidden.iter().any(|window| {
                window.address == bubble.hidden.address && window.monitor == bubble.hidden.monitor
            })
        });
        stale.for_each(|(_, bubble)| dismiss(bubble));
        for (slot, window) in hidden.into_iter().enumerate() {
            let bubble = self
                .shown
                .entry(window.address.clone())
                .or_insert_with(|| open(&self.app, self.corner, &window));
            bubble.revealer.set_tooltip_text(Some(&window.title));
            bubble.window.set_margin(self.corner.vertical, slot_margin(slot));
            bubble.hidden = window;
        }
    }
}

/// Open a bubble for `hidden` in `corner`, fading in.
fn open(app: &gtk::Application, corner: Corner, hidden: &Hidden) -> Bubble {
    let window = gtk::Window::builder().application(app).css_classes(["hyprhyde"]).build();
    window.init_layer_shell();
    window.set_namespace(Some("hyprhyde"));
    window.set_layer(Layer::Top);
    window.set_keyboard_mode(KeyboardMode::None);
    window.set_anchor(corner.vertical, true);
    window.set_anchor(corner.horizontal, true);
    window.set_margin(corner.horizontal, SCREEN_MARGIN);
    window.set_monitor(monitor_named(&hidden.monitor).as_ref());

    let image = icon::image_for_class(&hidden.class);
    image.set_pixel_size(BUBBLE_SIZE / 2);
    let button = gtk::Button::builder().child(&image).css_classes(["bubble"]).build();
    let (address, workspace) = (hidden.address.clone(), hidden.workspace);
    button.connect_clicked(move |_| {
        if let Err(error) = parking::restore(&address, workspace) {
            eprintln!("hyprhyde: restoring {address}: {error}");
        }
    });
    let revealer = gtk::Revealer::builder()
        .transition_type(gtk::RevealerTransitionType::Crossfade)
        .transition_duration(ANIMATION_MS)
        .child(&button)
        .build();
    window.set_child(Some(&revealer));
    window.present();
    revealer.set_reveal_child(true);
    Bubble { hidden: hidden.clone(), window, revealer }
}

/// Fade `bubble` out, then close its window.
fn dismiss(bubble: Bubble) {
    bubble.revealer.set_reveal_child(false);
    let window = bubble.window;
    glib::timeout_add_local_once(Duration::from_millis(ANIMATION_MS.into()), move || {
        window.destroy()
    });
}

/// Distance from the corner's vertical edge to the bubble in `slot` [px].
fn slot_margin(slot: usize) -> i32 {
    SCREEN_MARGIN + slot as i32 * (BUBBLE_SIZE + BUBBLE_GAP)
}

/// The GDK monitor whose connector is Hyprland's monitor `name`.
fn monitor_named(name: &str) -> Option<gdk::Monitor> {
    let monitors = gdk::Display::default()?.monitors();
    monitors
        .iter::<gdk::Monitor>()
        .filter_map(Result::ok)
        .find(|monitor| monitor.connector().as_deref() == Some(name))
}

/// Query Hyprland and update the bubbles.
fn refresh(bubbles: &RefCell<Bubbles>) {
    let hidden = hyprland::query::<ActiveWorkspace>("j/activeworkspace")
        .and_then(|active| Ok(hidden_on(&active, hyprland::query("j/clients")?)));
    match hidden {
        Ok(hidden) => bubbles.borrow_mut().show(hidden),
        Err(error) => eprintln!("hyprhyde: {error}"),
    }
}

/// Follow Hyprland's event socket, refreshing after each event that can move a bubble.
async fn follow_events(bubbles: Rc<RefCell<Bubbles>>) -> Result<(), glib::Error> {
    let path = hyprland::event_socket()
        .map_err(|error| glib::Error::new(gio::IOErrorEnum::NotFound, &error.to_string()))?;
    let connection =
        gio::SocketClient::new().connect_future(&gio::UnixSocketAddress::new(&path)).await?;
    let lines = gio::DataInputStream::new(&connection.input_stream());
    refresh(&bubbles);
    while let Some(line) = lines.read_line_utf8_future(glib::Priority::DEFAULT).await? {
        if hyprland::event_name(&line).is_some_and(|name| REFRESH_EVENTS.contains(&name)) {
            refresh(&bubbles);
        }
    }
    Err(glib::Error::new(gio::IOErrorEnum::Closed, "Hyprland closed the event socket"))
}

/// Run the daemon until Hyprland goes away; returns the process exit code.
pub fn run(corner: Corner) -> glib::ExitCode {
    let app = gtk::Application::builder().application_id("io.github.hyprhyde").build();
    app.connect_startup(move |app| {
        let provider = gtk::CssProvider::new();
        provider.load_from_string(CSS);
        gtk::style_context_add_provider_for_display(
            &gdk::Display::default().expect("a display"),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        let hold = app.hold();
        let bubbles =
            Rc::new(RefCell::new(Bubbles { app: app.clone(), corner, shown: HashMap::new() }));
        let app = app.clone();
        glib::spawn_future_local(async move {
            if let Err(error) = follow_events(bubbles).await {
                eprintln!("hyprhyde: {error}");
            }
            drop(hold);
            app.quit();
        });
    });
    app.connect_activate(|_| {});
    app.run_with_args::<&str>(&[])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hyprland::WorkspaceRef;

    fn client(address: &str, workspace: &str) -> Client {
        Client {
            address: address.into(),
            class: "kitty".into(),
            title: format!("title of {address}"),
            workspace: WorkspaceRef { id: workspace.parse().ok(), name: workspace.into() },
        }
    }

    #[test]
    fn only_windows_hidden_from_the_active_workspace_get_bubbles() {
        let active = ActiveWorkspace { id: 4, monitor: "DP-1".into() };
        let clients = vec![
            client("0x1", "special:hyprhyde-4"),
            client("0x2", "special:hyprhyde-5"),
            client("0x3", "4"),
            client("0x4", "special:scratchpad"),
            client("0x5", "special:hyprhyde-4"),
        ];
        let hidden = hidden_on(&active, clients);
        let addresses: Vec<&str> = hidden.iter().map(|window| window.address.as_str()).collect();
        assert_eq!(addresses, ["0x1", "0x5"]);
        assert!(hidden.iter().all(|window| window.workspace == 4 && window.monitor == "DP-1"));
    }

    #[test]
    fn corners_parse_from_their_names() {
        assert_eq!("bottom-right".parse(), Ok(Corner::default()));
        assert_eq!("top-left".parse(), Ok(Corner { vertical: Edge::Top, horizontal: Edge::Left }));
        for bad in ["", "bottom", "left-top", "bottom-middle"] {
            assert!(bad.parse::<Corner>().is_err(), "{bad}");
        }
    }

    #[test]
    fn bubbles_stack_away_from_the_corner() {
        assert_eq!(slot_margin(0), 16);
        assert_eq!(slot_margin(2), 16 + 2 * 52);
    }
}
