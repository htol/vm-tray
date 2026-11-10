# vm-tray

Tray icon for libvirt virtual machines. Clicking the icon opens a popup above
it with one row per VM: name, state, a Start/Stop button and a button that
opens `virt-viewer`.

- Tray: StatusNotifierItem, works with KDE Plasma and SNI hosts such as waybar.
- Popup: GTK4 with wlr-layer-shell, anchored next to the tray icon. Supported
  by KWin, niri, Hyprland and Sway.
- The popup is a short-lived process of the same binary (`vm-tray popup`), so
  GTK memory is only used while it is open.

Tested on KDE Plasma 6 (Wayland) only.

## Requirements

- Rust toolchain
- `gtk4`, `gtk4-layer-shell`, `libvirt`
- `virt-viewer`

## Install

```sh
make install
```

This installs into `~/.local` (override with `PREFIX=...`):

- `bin/vm-tray`
- `share/icons/hicolor/scalable/apps/vm-tray-symbolic.svg`
- `~/.config/systemd/user/vm-tray.service`, enabled for
  `graphical-session.target` and restarted

On niri and Hyprland the service starts only if the session activates
`graphical-session.target` (`niri-session`, uwsm).

## Usage

```
vm-tray [--connect URI[,URI...]] [--anchor top-right|top-left|bottom-right|bottom-left]
```

- `--connect` — libvirt URIs, default `qemu:///system,qemu:///session`.
- `--anchor` — popup corner when the tray host does not report the click
  position, default `bottom-right`.

In the popup:

- **Start** / **Stop** — `virDomainCreate` / graceful `virDomainShutdown`;
  **Resume** for paused VMs.
- **Viewer** — `virt-viewer --attach --connect URI NAME`, enabled for running
  VMs.

The popup closes on a second click on the tray icon.

## Icon

`data/vm-tray-symbolic.svg` is `virt-manager-symbolic` from
[KDE Breeze icons](https://invent.kde.org/frameworks/breeze-icons),
LGPL-3.0-or-later.
