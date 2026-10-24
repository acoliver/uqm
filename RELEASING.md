# Releasing UQM

The executable version lives in `rust/Cargo.toml` under `[workspace.package]`.
The `0.8.0` in content package names is the asset version, not the Rust port's
release version. Do not rename those packages when bumping the executable.

## Publish

1. Bump the workspace version on an issue branch, run the checks below, and merge
   the PR after CI passes. Commit the resulting `rust/Cargo.lock` changes too.
2. Tag that merged commit and push the tag, for example:

   ```sh
   git fetch origin
   git tag v0.9.0 origin/main
   git push origin v0.9.0
   ```

   The tag must exactly match the workspace version. For a prerelease, use a
   workspace version such as `0.9.0-rc.1` and tag `v0.9.0-rc.1`.
3. Watch the **Release** workflow. It creates a draft first, builds and tests every
   package, uploads nine packages and `SHA256SUMS`, then publishes the release.
   RC tags become GitHub prereleases and never update Homebrew. A failed build
   leaves a draft rather than publishing an incomplete release.

For a failed RC, fix and merge the code, bump to the next RC, and remove the failed
release and its tag. The manual workflow input accepts an existing version tag
and can resume a failed draft. It refuses to overwrite a published release.

For stable releases, add the repository Actions secret
`HOMEBREW_TAP_GITHUB_TOKEN` first. It needs Contents read/write permission on
`acoliver/homebrew-tap`. The final macOS job updates `Formula/uqm.rb` only after
all platforms pass. No tap token is needed for RCs. The normal repository
`GITHUB_TOKEN` creates the release and uploads assets.

## Packages

| Target | Runner | Packages |
| --- | --- | --- |
| macOS Apple Silicon | `macos-14` | `uqm-vVERSION-aarch64-apple-darwin.tar.gz` |
| Windows x64 | `windows-latest` | `uqm-vVERSION-x86_64-pc-windows-msvc.zip` |
| Windows ARM64 | `windows-11-arm` | `uqm-vVERSION-aarch64-pc-windows-msvc.zip` |
| Linux x64 | `ubuntu-24.04` | `uqm-vVERSION-x86_64-unknown-linux-gnu.{tar.gz,deb,rpm}` |
| Linux ARM64 | `ubuntu-24.04-arm` | `uqm-vVERSION-aarch64-unknown-linux-gnu.{tar.gz,deb,rpm}` |

Every package includes the tracked game content and license notices. Code is
GPL-2.0-or-later; game content is CC-BY-NC-SA-2.5 with the distribution permission
in `COPYING`. SDL2 is built from the crate's bundled source and statically linked.
Windows also statically links the VC runtime. The packaging job checks macOS
library paths and Windows PE imports to reject dependencies on builder-local
libraries.

The macOS archive contains `UQM.app`, ad-hoc signed, for macOS 11 or newer.
It is not notarized with an Apple Developer identity; Gatekeeper may require an
explicit user approval. Launch the app or run `UQM.app/Contents/MacOS/uqm`.
The Homebrew formula installs the app and a `uqm` command, for Apple Silicon only.

Unzip Windows and run `uqm.exe`. Linux tarballs contain `usr/bin/uqm` and
`usr/share/uqm`; keep that tree together and run `usr/bin/uqm`, or install the DEB
or RPM. Linux binaries are built on Ubuntu 24.04 and require glibc 2.39 or newer
and ALSA (`libasound2t64` on Ubuntu, `alsa-lib` on RPM distributions). SDL's video
and optional audio backends use the system's X11/Wayland/PulseAudio libraries.
Install the desktop runtime for the backend you use. This is not a static-libc
or older-distribution build.

Content is resolved beside the executable, inside the macOS app's Resources,
or at the executable prefix's `share/uqm`, before development working-directory
locations. An explicit `--contentdir=DIR` overrides discovery without falling back.
Config and saves retain their per-user locations: `~/.uqm` on macOS/Linux and
`%APPDATA%/uqm` on Windows. Packages never install or migrate user saves.

## Verification and local packaging

From `rust/`:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked --no-default-features --features audio_heart,debug-process -- -D warnings
cargo test --workspace --locked --no-default-features --features audio_heart,debug-process
cargo build --release --locked --bin uqm --features bundled-sdl
```

Run Rust commands from `rust/` so Cargo loads its SDL/CMake configuration.
CMake and a native compiler are required. Linux additionally needs ALSA and
SDL backend development headers, plus `dpkg-deb`, `rpmbuild` and `cpio` for
packaging. The workflow lists the Ubuntu packages.

From the repository root, using Python 3.11 or newer on the matching native host:

```sh
python3 -m unittest discover -s scripts/release -p 'test_*.py'
python3 scripts/release/package.py v0.9.0-rc.1 aarch64-apple-darwin
```

Outputs stay in `artifacts/release`. Every artifact is unpacked without installing
it into the builder's system. Its binary must report the exact version, print
help, open and decompress its packaged content resource map with `--check-content`,
and reject an invalid explicit content directory. The check verifies the actual
resolved path, runs outside the checkout and does not create user state. DEB and
RPM payloads get their own smoke tests. Normal CI also runs a native networking
smoke on all five targets and retains the existing gameplay scenario checks.

To inspect the stable Homebrew formula without publishing:

```sh
bash scripts/release/update_homebrew_tap.sh v0.9.0 MACOS_ARCHIVE_SHA256 > uqm.rb
brew style uqm.rb
```

The helper only writes to the tap when explicitly passed `--publish`; release CI
passes that flag for stable versions. Do not publish a stable tag until the tap
secret has been added.
