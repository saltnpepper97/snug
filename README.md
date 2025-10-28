<p align="center">
  <img src="assets/snug.png" alt="Snug Logo" width="200"/>
</p>

<h1 align="center">Snug</h1>

<p align="center">
 Wrap your workspace in comfort — Snug makes your desktop feel at home.
</p>

<p align="center">
  <img src="https://img.shields.io/github/last-commit/saltnpepper97/stasis?style=for-the-badge&color=%2328A745" alt="GitHub last commit"/>
  <img src="https://img.shields.io/aur/version/snug?style=for-the-badge" alt="AUR version">
  <img src="https://img.shields.io/badge/License-MIT-E5534B?style=for-the-badge" alt="MIT License"/>
  <img src="https://img.shields.io/badge/Wayland-00BFFF?style=for-the-badge&logo=wayland&logoColor=white" alt="Wayland"/>
  <img src="https://img.shields.io/badge/Rust-1.89+-orange?style=for-the-badge&logo=rust&logoColor=white" alt="Rust"/>
</p>

## Features
Snug allows you to create a sort of frame around your wayland session, be it square just to add colour from your theme or
to create the rounded corners effect of something like [quickshell](https://quickshell.org/) without the hastle of putting together a complex config.

- **Per-monitor configuration:** You can have seperate configurations for each of your monitors.
- **Colour theming:** RUNE makes it easy to create templates for say [pywal](https://github.com/dylanaraps/pywal) or [pywal16](https://github.com/eylles/pywal16)
- **Bar support:** Integrates well with existing bars such as [waybar](https://github.com/Alexays/Waybar), see [example script](https://github.com/saltnpepper97/snug/blob/main/examples/launch-snug-with-waybar.sh) for details.
>Important: Set "layer": "top" and "exclusive": false in your Waybar config. Also, reserve struts in your compositor (Hyprland/Niri) so Snug doesn’t overlap important UI areas

## Installation

### From source

```sh
git clone https://github.com/saltnpepper97/snug
cd snug
cargo build --release
sudo cp target/release/snug /usr/local/bin/snug
```

### AUR

For Arch linux users, you can install snug from the AUR via
```sh
yay -S snug
```
or for paru
```sh
paru -S snug
```

## Getting Started
If you just want to see it running or get things figured out I suggest first running
```
snug --help
```
This will give you a list of parameters, with this you can launch it with your parameters, by default it opens on all displays it can find.

If you just want to get going as right off the bat then copy this example config to XDG_CONFIG_HOME/snug/snug.rune.
The default config can also be found at `/usr/share/doc/snug/snug.rune`, copy it to `~/.config/snug/snug.rune`.
```

@author "Dustin Pilgrim"
@description "make your desktop snug with a nice frame :)"

# any display name will work
# HDMI-A-1, DP-1, DVI-D-1, etc.

# Primary display (DP-1)
DP-1:
  radius = 15
  left = 95
  right = 28
  top = 28
  bottom = 28
  color = "161b22"
  opacity = 1.0
end

# Secondary display (DP-2)
DP-2:
  radius = 15
  left = 95
  right = 30
  top = 30
  bottom = 30
  color = "161b22"
  opacity = 1.0
end
```
Use your compositors IPC to find your display name and replace or comment out the `DP-1` and `DP-2` blocks with your monitors names.

**Hyprland**

```
hyprctl monitors
```

**Niri**

```
niri msg output
```

Then within your compositors config in the autostart section make sure you add `snug` **BEFORE** your bar, e.g. [waybar](https://github.com/Alexays/Waybar) to have it load behind it.

### Starting with waybar

If you want a convenient script that starts/restarts both that you can call in your compositors autostart instead, use this following script

```bash
#!/usr/bin/env bash

# kill the old instance
pkill -x snug 2>/dev/null
pkill -x waybar 2>/dev/null

# start a new instance
snug >dev/null 2>&1 &
waybar >/dev/null 2>&1 &

exit 0
```

## Acknowledgements

- Thanks to [quickshell](https://quickshell.org/) for this lovely idea!
- Check out [waybar](https://github.com/Alexays/Waybar) for a solid compatible bar!
