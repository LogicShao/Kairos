# Kairos

> καιρός — the right, critical, or opportune moment

Cross-platform time management desktop app. Built with Tauri v2, Rust, and React.

## Features

| Module | Description |
|--------|-------------|
| 🍅 **Pomodoro** | Configurable work/break timer with circular progress ring and session logging |
| ✅ **Tasks** | Full CRUD with priority labels, status tracking, due dates, and filter/sort |
| 📅 **Courses** | Weekly grid schedule with color-coded courses and semester management |
| 📝 **Exams** | Countdown to exam dates with upcoming reminders |
| ☁️ **WebDAV Sync** | Self-hosted data sync via WebDAV protocol, LWW conflict resolution |

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop Shell | [Tauri v2](https://v2.tauri.app) |
| Frontend | React 19 + TypeScript + Tailwind CSS v4 + [shadcn/ui](https://ui.shadcn.com) |
| State | Zustand |
| Icons | [Lucide](https://lucide.dev) |
| Backend | Rust |
| Database | SQLite ([rusqlite](https://github.com/rusqlite/rusqlite) with bundled feature) |
| Sync | WebDAV ([reqwest](https://docs.rs/reqwest)) |

## Development

### Prerequisites

**Arch Linux:**
```bash
sudo pacman -S webkit2gtk-4.1 base-devel
```

**Windows 11:** WebView2 Runtime (pre-installed)

**Rust & Node:**
```bash
rustup default stable
cargo install tauri-cli --version "^2"
# Node.js >= 18 required
```

### Quick Start

```bash
# Install frontend dependencies
npm install

# Start dev server with hot reload
make dev
# or: cargo tauri dev
```

### Makefile Commands

```bash
make dev          # Start Tauri development server
make build        # Production build
make check        # Type-check Rust + TypeScript
make test         # Run all tests
make lint         # Lint Rust + TypeScript
make audit        # Offline compliance check
make verify       # Full CI pipeline (check + lint + test + audit)
make bump V=0.1.1 # Bump version across all config files
```

## Architecture

```
┌──────────────────────┐
│   React Frontend     │  TypeScript + Tailwind + shadcn/ui
├──────────────────────┤
│   Tauri IPC Bridge   │  #[tauri::command] ↔ invoke()
├──────────────────────┤
│   Rust Core Logic    │  timer, task management, schedule
├──────────────────────┤
│   SQLite Storage     │  rusqlite + versioned migrations
├──────────────────────┤
│   WebDAV Sync        │  reqwest + JSON file sync
└──────────────────────┘
```

Business logic lives entirely in Rust (`src-tauri/src/`). The React frontend is a replaceable rendering layer with zero business logic.

## Design

- **Offline-first** — no external network calls except WebDAV sync
- **Dark-first Fluent theme** — MS To Do inspired, periwinkle accent
- **System native fonts** — no external font CDN
- **Minimal dependencies** — no framer-motion, no external animation libraries

## License

MIT
