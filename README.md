# Inky

Inky is a Windows desktop focus assistant built with Tauri 2, React 18, and TypeScript.

The app provides a transparent always-on-top floating window with an animated octopus pet, quick task capture, focus sessions, local persistence, and optional AI task parsing through the Tauri backend.

## Development

```bash
corepack pnpm install
corepack pnpm dev
```

## Checks

```bash
corepack pnpm typecheck
corepack pnpm build
cargo check --manifest-path src-tauri/Cargo.toml
```

## Desktop app

```bash
corepack pnpm tauri dev
```
