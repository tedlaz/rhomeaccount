# rhomeaccount

Personal home accounting — a small double entry book keeping desktop app.
Rust/egui port of the PySide6 `qhomeacc`, on top of a core library ported from
the Python `qlogistiki` package.

A *book* is a plain text folder: a `000` metadata file that declares the chart
of accounts, plus one journal file per period. The app reads it and shows the
trial balance as a collapsible account tree on the left, and the selected
account's card (*kartella*) with a chart, date filters and a balance check
report on the right.

## Download

Grab the latest build from the [releases page](https://github.com/tedlaz/rhomeaccount/releases/latest).

| Platform | File | How to run |
| --- | --- | --- |
| Windows | `rhomeaccount-<version>-windows-x86_64.zip` | Unzip, run `rhomeaccount.exe`. |
| Linux | `rhomeaccount-<version>-x86_64.AppImage` | `chmod +x rhomeaccount-*.AppImage && ./rhomeaccount-*.AppImage` |
| Linux | `rhomeaccount-<version>-x86_64.flatpak` | `flatpak install --user ./rhomeaccount-*.flatpak` then `flatpak run io.github.tedlaz.rhomeaccount` |

The AppImage is built on Ubuntu 22.04, so it needs glibc 2.35 or newer. On an
older distribution use the Flatpak, which carries its own runtime.

## Building from source

Needs a stable Rust toolchain.

```sh
cargo build -r
```

The release profile is tuned for size (`opt-level = "z"`, fat LTO, one codegen
unit, no unwinding tables), so a release build is noticeably slower than a
debug one. For day to day work use `cargo run -p rhomeaccount`.

On Debian/Ubuntu the GUI needs these development packages:

```sh
sudo apt-get install -y pkg-config libx11-dev libxrandr-dev libxi-dev \
  libxcursor-dev libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev \
  libgl1-mesa-dev libegl1-mesa-dev
```

File dialogs go through the XDG desktop portal, so no GTK development
packages are required.

Tests live in the core crate and include a golden file comparison against the
Python implementation's output:

```sh
cargo test
```

## Layout

```
core/        accounting library — parser, book, chart of accounts, transactions
app/         egui front end, fonts and icons
packaging/   .desktop entry, AppStream metainfo, Flatpak manifest
.github/     release workflow
```

Settings are stored per user (`config_dir/TedLazaros/rhomeaccount/settings.json`).

## Releasing

Pushing a tag that starts with `v` runs
[`.github/workflows/release.yml`](.github/workflows/release.yml), which builds
the three artifacts — Windows zip, AppImage, Flatpak bundle — and publishes
them on a GitHub release.

The same workflow can be started by hand from the Actions tab. A manual run
builds all three packages and leaves them as workflow artifacts without
touching releases — the cheap way to test a packaging change before tagging.

Note that the Flatpak is built with the network switched off, the way
flatpak-builder always builds. The workflow vendors the crates in a separate
job and hands them over as an artifact, so `Cargo.lock` must be committed and
in sync or the build fails on `--locked`.

### 1. Start from a clean `main`

```sh
git switch main
git pull
git status
```

There is no CI workflow yet, so nothing has checked the branch for you —
step 3 is that check.

### 2. Bump the version

Both crates inherit their version from the workspace, so one field moves them.
Using `0.2.0` as the example:

1. The workspace `Cargo.toml` — the `version` field of `[workspace.package]`:

   ```toml
   [workspace.package]
   version = "0.2.0"
   ```

2. `Cargo.lock` — refresh it so the new version lands in the lock file. The
   release build uses `--locked` and fails if the two disagree:

   ```sh
   cargo check --workspace
   ```

3. `packaging/linux/io.github.tedlaz.rhomeaccount.metainfo.xml` — add a
   `<release>` entry at the top of `<releases>`, with today's date in
   `YYYY-MM-DD`:

   ```xml
   <release version="0.2.0" date="2026-09-01">
     <description>
       <p>What changed in this version.</p>
     </description>
   </release>
   ```

   Software centres (GNOME Software, KDE Discover) read this file, so a
   missing entry means the update shows up without a changelog.

### 3. Run the checks locally

Faster than finding out from CI:

```sh
cargo fmt --all --check
cargo clippy --all-targets
cargo test --workspace
```

The core crate still has four clippy warnings, so `-D warnings` would fail
today — clear them first if you want that in the loop.

### 4. Commit and tag

Nothing enforces that the tag matches the version in `Cargo.toml`, and the
artifact names follow the tag, so a mismatch just produces confusingly named
files. Keep them the same, with a `v` in front of the tag.

```sh
git add Cargo.toml Cargo.lock packaging/linux/io.github.tedlaz.rhomeaccount.metainfo.xml
git commit -m "Release 0.2.0"
git tag -a v0.2.0 -m "rhomeaccount 0.2.0"
git push origin main --follow-tags
```

### 5. Watch the build

```sh
gh run watch
```

Or open the **Actions** tab. Six jobs run: `setup` decides the version string,
`windows` and `appimage` build in parallel, `vendor` collects the crates for
`flatpak`, and `release` gathers the three artifacts and publishes them.
Expect roughly 10–20 minutes; the size-tuned release profile (fat LTO, one
codegen unit) is the slow part.

## After the release

Once the workflow has finished and the release page is up:

- [ ] Download each of the three artifacts and actually launch them — a
      packaging break only shows up on a clean machine, never in CI.
- [ ] Check that the Flatpak can still open a book folder outside `$HOME`
      and write its settings file.
- [ ] Edit the generated release notes into something a user can read —
      the auto-generated commit list is a starting point, not the notes.
- [ ] Add or refresh a screenshot in this README if the UI changed — there is
      none yet, and the download table is the first thing people read.
- [ ] Skim the workflow logs for new deprecation warnings from the actions;
      they are warnings for a few months and then a hard failure.
