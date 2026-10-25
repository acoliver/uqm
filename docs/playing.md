# Play

Launch `UQM.app` on macOS, `uqm.exe` on Windows, or `uqm` on Linux/Homebrew.
See [installation](install.md) or [building](building.md) if you do not yet have
an executable. Choose **New Game** for the campaign, **Load Game** to continue,
**Super Melee!** for fleet battles, or **Setup** to change settings and controls.

The [game manual](manual.txt) includes the story, exploration, combat and ship
reference. Its text is preserved from upstream v0.7. Its installation advice,
command-line defaults and platform details are historical; use these guides and
`uqm --help` for the Rust port.

## Controls

These are the shipped keyboard bindings. Setup lets you select and edit ship
control templates, including joystick templates.

| Action | Default keys |
| --- | --- |
| Navigate menus | Arrow keys |
| Select | Enter or Right Ctrl |
| Cancel/open the in-game menu | Space, Escape or Right Shift |
| Turn ship | Left/Right arrows |
| Thrust | Up arrow |
| Fire | Right Ctrl (also Enter on the arrow template) |
| Ship special ability | Right Shift (also keypad 0) |
| Emergency warp escape in combat | Escape |
| Pause | F1 or Pause |
| Toggle fullscreen | F11 |
| Exit prompt | F10 |
| Starmap zoom | Page Up/Page Down |

The WASD ship template uses W/A/S/D with V for weapon and B for special.
Ship controls and menu controls are separate. The Game menu provides save/load
commands during the campaign. See the manual for lander and Super Melee controls.

## Command-line options

Run `uqm --help` for the full list and `uqm --version` for the executable version.
The following options are listed by the current executable's help:

| Option | Use |
| --- | --- |
| `--res=WIDTHxHEIGHT` | Window/display dimensions; compiled default 1280x960 |
| `--fullscreen`, `--windowed` | Fullscreen or windowed mode; windowed is the default |
| `--scale=hq`, `--scale=xbrz3`, `--scale=xbrz4`, `--scale=none` | Scaling mode; default xbrz3 |
| `--meleezoom=step`, `--meleezoom=smooth` | Combat zoom style; default smooth (3DO) |
| `--musicvol=0` through `--musicvol=100` | Music volume |
| `--sfxvol=0` through `--sfxvol=100` | Sound-effects volume |
| `--speechvol=0` through `--speechvol=100` | Speech volume |
| `--nosubtitles` | Disable subtitles |
| `--intro=pc`, `--intro=3do` | Intro/ending presentation choice |
| `--configdir=DIR` | Use a separate configuration and save tree |
| `--contentdir=DIR` | Select the content root containing `version` and `packages/` |
| `--check-content` | Validate the base content package and exit without opening user state |
| `--addondir=DIR` | Use another add-on directory |
| `--addon NAME` | Load a named add-on; repeat for multiple add-ons |
| `--logfile=FILE` | Send tracing output to a file |
| `--safe` | Skip saved configuration settings |

For example:

```sh
uqm --windowed --res=1280x960 --musicvol=70
```

Command-line settings override saved settings, which override compiled defaults.
For a macOS app installed manually, replace `uqm` with the path to
`UQM.app/Contents/MacOS/uqm`. In Windows PowerShell, use `.\uqm.exe`.

## Configuration and saves

| Platform | Configuration directory | Campaign saves | Melee teams |
| --- | --- | --- | --- |
| macOS | `~/.uqm` | `~/.uqm/save` | `~/.uqm/teams` |
| Linux | `~/.uqm` | `~/.uqm/save` | `~/.uqm/teams` |
| Windows | `%APPDATA%\uqm` | `%APPDATA%\uqm\save` | `%APPDATA%\uqm\teams` |

On Windows, `%APPDATA%` normally means your user's `AppData\Roaming` directory.
The code uses `%USERPROFILE%\AppData\Roaming` if `APPDATA` is absent. The main
settings file is `uqm.cfg`. Shipped controls come from the content files
`uqm.key` and `menu.key`; user ship bindings are saved in `flight.cfg` and menu
overrides are read from `override.cfg`.
Back up the configuration directory to preserve settings, saves and teams.
These directories are separate from the installed game content.

`--configdir` overrides the `UQM_CONFIG_DIR` environment variable and the platform
default. Use it for separate profiles. Do not point it at the game content tree.

## Add-ons

Release packages and this repository include these add-ons under the content
root's `addons/` directory:

| Directory | Content |
| --- | --- |
| `3domusic` | 3DO music tracks |
| `3dovoice` | 3DO voice recordings |
| `3dovideo` | 3DO video resources |
| `remix` | Official Precursors music remixes |

Setup controls 3DO music, remix music, speech and presentation choices. Startup
loads the corresponding named add-ons when those options are enabled; 3DO video
is selected by `--intro=3do`. Music, remix and speech are enabled in the compiled
defaults, and the default intro choice is 3DO. Saved settings can change them.

For other compatible packs, keep their directory structure under `addons/NAME`
(or under a directory selected by `--addondir`) and load them with `--addon NAME`.
The loader supports `.uqm` and `.zip` archives as well as loose files. Add-ons
override resources; keep a copy of the original content when trying replacements.
