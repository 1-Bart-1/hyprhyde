# hyprhyde

Collapse Hyprland windows into small floating bubbles; click a bubble to bring the window back.

No bar and no tray: each hidden window becomes one round bubble showing its app icon, stacked in a corner of the screen.

## Features

- `hyprhyde hide` collapses the active window into a 44 px bubble in the bottom-right corner.
- Clicking a bubble brings the window back onto the workspace you are on and focuses it.
- Bubbles belong to the workspace the window was hidden from: switch workspace and you see that workspace's bubbles instead. With no bubbles nothing is on screen and nothing takes clicks.
- On several monitors the bubbles show on the focused monitor, for its active workspace.
- The window title is the bubble's tooltip; bubbles fade in and out and light up on hover (Catppuccin Mocha colours).
- No state file: a hidden window waits in the special workspace `special:hyprhyde-<workspace name>`, so a restarted daemon shows the bubbles left from before.

## Dependencies

| Type    | Packages                                  |
|---------|-------------------------------------------|
| Runtime | `hyprland` (>= 0.56) `gtk4` `gtk4-layer-shell` |
| Build   | `cargo`                                   |

## Install

### From source

```sh
make build
make install   # installs to ~/.local/bin/
```

### Arch Linux

A `PKGBUILD` in `arch/` builds a package from this repository:

```sh
cd arch
makepkg -si
```

## Usage

```
hyprhyde <command>
```

| Command  | Description                                                  |
|----------|--------------------------------------------------------------|
| `hide`   | Collapse the active window into a bubble                     |
| `daemon` | Show the bubbles and follow Hyprland's events                |
| `help`   | Show usage                                                   |
| `version`| Print the version                                            |

`hyprhyde daemon --corner <corner>` puts the bubbles in another corner: `bottom-right` (default), `bottom-left`, `top-right` or `top-left`.

Hiding does nothing when no window is focused or when the focused window is on a special workspace.

## Hyprland integration

hyprhyde talks to Hyprland over its UNIX sockets: `.socket.sock` for queries and dispatches, `.socket2.sock` for events. Hyprland 0.56 replaced the string dispatch API with a Lua one, so hyprhyde requires 0.56 or newer.

Keybind in your `hyprland.lua`:

```lua
hl.bind("SUPER + H", hl.dsp.exec_cmd("hyprhyde hide"))
```

Start the daemon on login:

```lua
hl.exec_cmd("hyprhyde daemon")
```

## License

MIT
