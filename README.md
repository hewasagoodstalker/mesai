# Mesai — Study Tracker

A small desktop app for tracking when, how long and on what you've been working. Written in Rust; the UI (eframe/egui) talks **Wayland natively** through winit and falls back to X11 when Wayland isn't available.

The interface is **English by default**; Turkish can be selected from the language switcher in the top-right corner (the choice is persisted).

## Features

- **Timer:** Start, pause, resume, finish. Time is measured with the wall clock, so it stays correct even if your laptop suspends mid-session.
- **Session log:** When you stop the timer you enter a topic, notes and tags.
- **Attachments:** Attach the files you worked on (e.g. your `.c` sources) to a session. Files are *copied*, so you keep that day's version even if the original changes or gets deleted later. Three ways to add them:
  - **"Add files…"** — opens your desktop's native file dialog through the XDG desktop portal. This is the primary method and works everywhere, including Wayland.
  - **Typing a path** into the text field (`~` is expanded).
  - **Drag & drop** onto the window — X11 only: winit's Wayland backend does not deliver file-drop events, which is why the portal-based picker exists.
- **Stats:** Total time, average session, day streak, a last-14-days chart, per-topic breakdown.
- **Export & import (Data tab):**
  - **CSV** for spreadsheets. Headers are always English so exported data stays stable regardless of UI language.
  - **JSON** for backups. Attachment files up to **5 MB are embedded** (base64) in the backup, so it restores completely even on a fresh machine; larger files are referenced by path only and the app tells you how many were skipped.
  - **Import** merges a JSON backup into your data: existing sessions are kept, duplicates (same start & end time) are skipped, imported sessions get fresh IDs, and attachments are restored from the embedded bytes — or copied from the original path as a fallback for old-format backups. On an empty database, import is a full restore.

## Requirements

A Rust toolchain (1.79+; easiest via [rustup](https://rustup.rs)):

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Runtime libraries — already installed on virtually every desktop distro:

```sh
# Debian / Ubuntu
sudo apt install libxkbcommon0 libegl1

# Fedora
sudo dnf install libxkbcommon mesa-libEGL

# Arch
sudo pacman -S libxkbcommon mesa
```

The file picker uses the **XDG desktop portal** (`xdg-desktop-portal` plus a backend such as `-gtk`, `-kde`, `-wlr` or `-hyprland`). Every mainstream Wayland desktop ships one; on minimal window-manager setups make sure a portal backend is installed and running.

## Build, test & run

```sh
cd mesai
cargo test          # backup/restore pipeline tests
cargo run --release
```

The compiled binary is `target/release/mesai`; copy it into `~/.local/bin/` if you want to launch it directly. `Cargo.lock` pins the exact dependency versions this code was verified against.

## Where is my data?

```
~/.local/share/mesai/
├── data.json          # all sessions + settings (readable JSON, never contains base64)
└── attachments/<id>/  # copies of each session's attached files
```

Exports land in `~/Documents` (or your home directory if it doesn't exist) as `mesai_DATE.csv` / `.json`.

## Wayland notes

- The window's `app_id` is `mesai`; use it in window rules on Sway/Hyprland etc.
- The app opens on Wayland when `WAYLAND_DISPLAY` is set; force X11 with `WAYLAND_DISPLAY= cargo run --release` (drag & drop works there).

## Ideas if you want to extend it

The code is a single commented file (`src/main.rs`) split into clear sections — easy to poke at. Good exercises: adding a session manually without the timer, editing saved sessions, weekly goals, or migrating storage to SQLite. The `Strings` struct also makes adding a third language trivial: the compiler will tell you exactly which strings are missing.
