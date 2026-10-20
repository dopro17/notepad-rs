# notepad-rs

A simple, Notepad-like text editor for macOS, written in Rust.

I was searching for a Notepad-like app for macOS but found no good candidates: some of them are too complex, others just don't work as expected. So I made this one.

- **Notepad-like**: open, edit, save. Nothing more.
- **Written in Rust** ([egui](https://github.com/emilk/egui) + [muda](https://github.com/tauri-apps/muda) for the native macOS menu bar).
- **AI-generated**: the whole codebase was written by [Claude Code](https://claude.com/claude-code).
- **Note:** I don't even know Rust, so I didn't touch the code.

> ### For any fork: **KEEP IT SIMPLE, AS IT IS!**

## Features

- Tabs; New / Open / Save / Save As; asks before closing unsaved work
- Undo / redo, cut / copy / paste, select all
- Find / replace (case sensitive, wrap around), go to line, insert time/date (F5)
- Word wrap, font (monospace / proportional, size), zoom (⌘+ / ⌘- / ⌘0, ⌘ + scroll)
- Status bar: line / column, zoom, line ending (CRLF / LF / CR), encoding
- Detects and preserves the encoding (UTF-8, UTF-8 BOM, UTF-16 LE/BE, Windows-1252) and the line endings
- Drag & drop files, "Open With" from Finder, settings remembered between sessions
- Native macOS menu bar with the usual shortcuts

macOS is the main target. Windows and Linux builds are kept compiling and tested in CI.

## Install (macOS)

Download `notepad-rs-macos.zip` from the latest [CI run](../../actions) artifacts, unzip it and move `notepad-rs.app` to `/Applications`.

The app is not signed by Apple, so Gatekeeper may block it. To allow it:

```bash
xattr -cr /Applications/notepad-rs.app
```

## Build from source

Requires [Rust](https://rustup.rs) and the Xcode Command Line Tools.

```bash
cargo run                              # run in debug mode
cargo install cargo-bundle             # once
./scripts/build-macos-universal.sh     # builds target/universal/notepad-rs.app (Apple Silicon + Intel)
```

Linux needs: `build-essential pkg-config libgtk-3-dev libxkbcommon-dev libwayland-dev libssl-dev`.

## Development

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```

See [CLAUDE.md](CLAUDE.md) for the project rules and structure.
