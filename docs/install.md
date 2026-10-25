# Install

Download packages from [GitHub Releases](https://github.com/acoliver/uqm/releases).
Choose a stable release for normal play, or a prerelease if you want to test a
release candidate. The release workflow produces the packages below and a
`SHA256SUMS` file. Replace `VERSION` in filenames with the release version.

Packages include game content, bundled SDL2 and license notices. You do not need
to install SDL2 separately to run a release. Your configuration and saves stay
in your user directory; installing a package does not move or overwrite them.
See [playing](playing.md#configuration-and-saves) for their locations.

## macOS

The release supports Apple Silicon on macOS 11 or newer. The Homebrew tap formula
ships with the first stable release. Once it is published:

```sh
brew install acoliver/tap/uqm
uqm
```

Alternatively, extract `uqm-vVERSION-aarch64-apple-darwin.tar.gz` and move
`UQM.app` to Applications or another folder. Open the app, or run
`UQM.app/Contents/MacOS/uqm` from a terminal. Content lives inside the app at
`Contents/Resources/content`; leave that directory intact.

The app is ad-hoc signed, not Apple-notarized. macOS may require you to approve
it in System Settings > Privacy & Security after the first launch attempt.
No Intel macOS release package is currently produced; see [building](building.md).

## Windows

Download the ZIP matching your machine:

- x64: `uqm-vVERSION-x86_64-pc-windows-msvc.zip`
- ARM64: `uqm-vVERSION-aarch64-pc-windows-msvc.zip`

Extract the entire ZIP into a folder you can keep, then run `uqm.exe`.
Keep the adjacent `content` directory with the executable. SDL2 and the Visual C++
runtime are statically linked in these packages.

## Linux

Packages are available for x64 (`x86_64-unknown-linux-gnu`) and ARM64
(`aarch64-unknown-linux-gnu`) as `.tar.gz`, `.deb` and `.rpm`.

For a tarball, extract `uqm-vVERSION-TARGET.tar.gz`, keep its `usr` tree together,
and run `./usr/bin/uqm` from the extracted directory. Content is at
`usr/share/uqm`.

For a system installation, install the matching `.deb` with your distribution's
APT tools, or the `.rpm` with DNF, then run `uqm`. These packages put the binary
in `/usr/bin/uqm` and content in `/usr/share/uqm`.

The binaries are built on Ubuntu 24.04 and require glibc 2.39 or newer and ALSA
(`libasound2t64` on Ubuntu, `alsa-lib` on RPM distributions). The desktop's
X11/Wayland and optional PulseAudio runtime libraries supply SDL's backends.
For older distributions, [build from source](building.md) instead.

## Content and troubleshooting

Content includes `version`, the base package in `packages/`, and add-ons in
`addons/`. The executable finds the installed content relative to its own path.
You can check it without opening a window or touching saves:

```sh
uqm --check-content
```

If using another content tree, pass `--contentdir=/path/to/content`. Choose the
content root containing `version` and `packages`, not the `packages` directory
itself. An invalid explicit path fails rather than falling back to another copy.

Continue with [launching, controls and add-ons](playing.md).
