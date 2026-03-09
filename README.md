# Iris

A GNOME application for managing [ReShade](https://reshade.me/) under Wine/Proton on Linux.

Iris handles downloading ReShade, installing it into your game directories, and managing shader repositories — all from a native GTK4/libadwaita interface.

## Features

- **Game management** — auto-discovers Steam games; supports manual game entries
- **ReShade installation** — installs ReShade into any Wine/Proton game directory with the correct DLL override and architecture (x86 / x86_64)
- **Version cache** — download and keep multiple ReShade versions locally; supports both Standard and Addon Support variants independently
- **Shader catalog** — 40+ curated community shader repositories, cloneable with one click; optional shader merging into a single directory for easy ReShade path configuration
- **Custom repos** — add any Git repository as a shader source
- **Auto update check** — checks GitHub for new ReShade releases on a configurable interval (default: 4 h)
- **Global ReShade.ini** — optionally share one config file across all games

## Requirements

- Linux with Wine or Proton
- GNOME / GTK4 runtime (`libgtk-4`, `libadwaita`)

## Acknowledgements

- [reshade-steam-proton](https://github.com/kevinlekiller/reshade-steam-proton) by kevinlekiller — the original shell script that pioneered ReShade management under Wine/Proton on Linux, and the inspiration for this project.
- [ReShade](https://reshade.me/) by crosire.
- [ratic](https://gitlab.gnome.org/ratcornu/ratic) by ratcornu — GTK4/Relm4 application template this project is based on.

## License

GPL-2.0
