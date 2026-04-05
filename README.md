# LocalOpenRouter

LocalOpenRouter is a local AI proxy for developer tooling. This MVP ships:

- A Rust daemon that listens on `127.0.0.1:7331`
- Built-in `codex`, `claude-code`, and `gemini` providers plus user-defined custom providers
- Dynamic provider namespaces such as `/openai/*`, `/anthropic/*`, or any custom proxy path
- Provider protocol profiles for `openai`, `anthropic`, and `generic`
- A master-password-protected local account vault
- SQLite-backed accounts, route bindings, and full request logs
- A Tauri desktop shell for unlock, account management, routing, onboarding, and log viewing

## Run

Start the daemon directly:

```bash
cargo run -p localopenrouter-daemon
```

Start the desktop shell:

```bash
cargo run -p localopenrouter
```

For UI-only work, you can skip repeated frontend asset recompiles by enabling the lightweight
source-backed dev server:

```bash
LOCALOPENROUTER_UI_DEV=1 cargo run -p localopenrouter
```

In that mode the desktop window loads `apps/desktop/ui` directly from disk and auto-reloads when
`html`, `css`, or `js` files change. Rust changes still require a normal rebuild.

The Tauri app will try to launch `localopenrouter-daemon` automatically from the workspace `target` directory. You can override the daemon path with:

```bash
export LOCALOPENROUTER_DAEMON_PATH=/absolute/path/to/localopenrouter-daemon
```

Data is stored under `LOCALOPENROUTER_DATA_DIR` when set, otherwise the platform local data directory is used.
