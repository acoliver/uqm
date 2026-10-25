# Build from source

Install [Rust through rustup](https://rustup.rs/), Git, CMake and a native compiler.
CI currently uses Rust 1.98.0. The game is Rust; a C/C++ toolchain is still needed
to build the external SDL2 library from its bundled source.

Clone the repository, including its tracked content:

```sh
git clone git@github.com:acoliver/uqm.git
cd uqm/rust
```

Run Cargo from `rust/` so it loads `.cargo/config.toml`, including the CMake
compatibility setting for bundled SDL2. `--locked` uses the committed dependency
versions.

## macOS

Install the Xcode Command Line Tools (`xcode-select --install`) and CMake
(for example, `brew install cmake`). Then, from `rust/`:

```sh
cargo build --release --locked --bin uqm --features bundled-sdl
./target/release/uqm --contentdir=../sc2/content
```

## Linux

Install your distribution's compiler toolchain, CMake, pkg-config, ALSA headers
and the SDL backend headers. On Ubuntu 24.04, the release workflow uses these
backend dependencies:

```sh
sudo apt-get install build-essential cmake pkg-config libasound2-dev \
  libx11-dev libxcursor-dev libxrandr-dev libxi-dev libxss-dev libxext-dev \
  libwayland-dev libxkbcommon-dev libpulse-dev
```

From `rust/`:

```sh
cargo build --release --locked --bin uqm --features bundled-sdl
./target/release/uqm --contentdir=../sc2/content
```

## Windows

Install Visual Studio Build Tools with the Desktop development with C++ workload
and a Windows SDK. Install CMake and use an MSVC Rust toolchain for your machine
(x64 or ARM64). From PowerShell in `rust/`:

```powershell
$env:RUSTFLAGS = "-C target-feature=+crt-static"
cargo build --release --locked --bin uqm --features bundled-sdl
.\target\release\uqm.exe --contentdir=..\sc2\content
```

The `RUSTFLAGS` setting matches the release workflow's static Visual C++ runtime.
The `bundled-sdl` feature builds and statically links SDL2 on all three platforms.
You do not need a separate SDL2 installation for that build. It does not make
Linux's libc, ALSA or desktop backend libraries static.

## Use an installed SDL2 instead

On macOS/Linux, you can omit `--features bundled-sdl` after installing SDL2
headers/libraries and pkg-config. For example, `brew install pkg-config sdl2`
on macOS, or `sudo apt-get install libsdl2-dev pkg-config libasound2-dev` on Ubuntu.
The build script discovers SDL2 through pkg-config. Release packages use the
bundled/static route above.

## Run from the repository

For a development build, from `rust/`:

```sh
cargo run --locked --bin uqm --features bundled-sdl -- --contentdir=../sc2/content
```

The source content root is `sc2/content`. It contains the `version` marker,
`packages/uqm-0.8.0-content.uqm` and the `addons/` tree. Pass that root rather
than `sc2/content/packages`. Paths passed on the command line are relative to
your current directory. Normal runs use your usual saves; pass
`--configdir=/path/to/separate-config` if you want a separate game profile.

See [playing](playing.md) for controls and options. Contributors should also read
[the development guide](../dev-docs/README.md) for checks and scenario tooling.
