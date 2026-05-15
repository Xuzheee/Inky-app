# AGENTS.md

This file provides guidance to Codex (Codex.ai/code) when working with code in this repository.

## Project overview

Inky is a Windows desktop focus assistant built with Tauri 2, Rust, React 18, TypeScript, and Vite. It runs as a transparent, borderless, always-on-top desktop widget for task capture, focus/pomodoro flow, mini pet mode, local persistence, and optional AI task parsing.

## Commands

- Install dependencies: `corepack pnpm install`
- Run full desktop app in development: `corepack pnpm tauri dev`
- Run frontend-only browser preview: `corepack pnpm dev`, then open `http://127.0.0.1:1420`
- Type-check frontend: `corepack pnpm typecheck`
- Build frontend: `corepack pnpm build`
- Check Rust/Tauri backend: `cargo check --manifest-path src-tauri/Cargo.toml`
- Run Rust backend tests: `cargo test --manifest-path src-tauri/Cargo.toml`
- Run one Rust test by name: `cargo test --manifest-path src-tauri/Cargo.toml <test_name>`
- Build desktop app package: `corepack pnpm tauri build`

`package.json` currently contains `test:proxy`, but its target file `api/parse-task.test.js` is not present in the repository.

## Architecture

The frontend entry point is `src/main.tsx`, which renders `src/App.tsx`; `App` is intentionally thin and hosts `FocusFlowWidget`. Most user-facing state and flow lives in `src/components/FocusFlowWidget/FocusFlowWidget.tsx`: task CRUD, mood, XP, pomodoro timing, mini mode, overlays, reminders, and AI settings UI.

Shared frontend domain types are in `src/types/index.ts`. Keep task categories, priorities, mood values, and overlay/view states aligned with Rust validation when changing these unions.

Frontend persistence and AI calls are wrappers around Tauri commands:

- `src/utils/focusFlowPersistence.ts` normalizes persisted focus-flow state and invokes `load_focus_flow_state` / `save_focus_flow_state`.
- `src/utils/aiParser.ts` invokes `parse_task_with_ai`, imposes a 5s frontend timeout, and normalizes model/proxy results.
- `src/utils/aiSettings.ts` invokes commands for AI config status, saving/clearing config, and app-local data path discovery.

The pet display is isolated in `src/components/PetRenderer/`; pet asset level/expression mappings live in `petConfig.ts`, and image files are loaded from `public/pet-assets/` by public path rather than TypeScript imports.

The Tauri backend is in `src-tauri/src/`:

- `main.rs` registers commands, configures the tray menu, Alt+F global shortcut, window drag/mini-size commands, and Windows idle-time detection.
- `persistence.rs` owns the SQLite schema and state validation for tasks, mood, XP, pet name, and pomodoro counts. It stores data in `focusflow.sqlite3` under the app data directory.
- `Codex.rs` owns optional AI parsing configuration and network calls. Personal DeepSeek keys and invite proxy config are stored in `ai-config.json` under the app local data directory; the frontend should only receive config status, never secret values.

`src-tauri/tauri.conf.json` is the source of truth for the desktop shell: it runs `corepack pnpm dev` before Tauri dev, uses `http://127.0.0.1:1420`, runs `corepack pnpm build` before packaging, and defines the default 300x520 transparent always-on-top window.

## Development notes

- Use `corepack pnpm tauri dev` when testing desktop-only behavior such as window dragging, tray/global shortcut behavior, idle detection, app data paths, SQLite persistence, or Tauri command integration.
- Browser-only `corepack pnpm dev` is useful for visual frontend iteration, but Tauri native capabilities will be unavailable or best-effort-failing there.
- When changing persisted state shape, update both the TypeScript normalizers and Rust DTO/schema validation together.
- When changing AI parsing result shape, update Rust normalization in `Codex.rs` and frontend normalization in `aiParser.ts` together.
- Keep API keys and invite codes in the Tauri backend/app-local config path; do not add frontend code paths that expose raw secrets.
