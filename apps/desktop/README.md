# ProcGeo Desktop

Native desktop app for ProcGeo, built with [Tauri](https://tauri.app).

Unlike the web playground (which runs procgeo compiled to WebAssembly), this app
links **procgeo as a native Rust dependency**. SOP graphs are cooked on the
native side at full speed and only render-ready buffers cross the IPC boundary
into the Three.js webview.

```
apps/desktop/
  index.html, src/        Frontend (Vite + Three.js, runs in the system webview)
  src-tauri/              Rust backend
    src/lib.rs            Tauri commands: cook(graph), list_sops()
    Cargo.toml           Depends on procgeo-core + procgeo-sops (path deps)
```

The `src-tauri` crate is intentionally **detached** from the root Cargo
workspace (it carries its own empty `[workspace]`), so Tauri's GUI dependency
tree stays out of `cargo build --workspace` and CI.

## Prerequisites

- Rust toolchain (stable)
- On Windows: [WebView2](https://developer.microsoft.com/microsoft-edge/webview2/)
  (preinstalled on Windows 11) and the MSVC build tools
- Node deps installed from the repo root: `pnpm install`

## Develop

From the repo root:

```bash
pnpm dev:desktop      # = pnpm --filter @procgeo/desktop dev = tauri dev
```

This launches Vite (port 5174) and the native window with hot reload. The first
run compiles the full Tauri dependency tree and will take a few minutes.

## Build a release binary

```bash
pnpm build:desktop    # = tauri build
```

Output (installer + executable) lands in
`apps/desktop/src-tauri/target/release/bundle/`.

## Icons

Placeholder icons live in `src-tauri/icons/`. To regenerate from a source PNG:

```bash
pnpm --filter @procgeo/desktop tauri icon path/to/logo.png
```

## Adding a SOP to the app

No backend change is needed for new SOPs — `cook` dispatches through
`procgeo_sops::default_registry()`, so any registered SOP is callable by name.
Add a button in `src/main.js` with the SOP name and params to expose it.
