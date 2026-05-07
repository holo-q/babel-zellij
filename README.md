# babel-zellij

Zellij plugin for [babel](https://github.com/holo-q/babel) — live agent session status in your terminal multiplexer.

## What it does

Renders a status bar pane inside zellij showing the state of all tracked agent sessions (Claude, Codex, Gemini, etc.) managed by babel. Updates are push-based via zellij pipe IPC — no polling.

```
●●○ 2 working | 1 await | 3 tracked │ ●c ◐x ○g
```

## Architecture

```
babel daemon ──paint stream──→ babel zellij-bridge ──zellij pipe──→ this plugin
```

1. `babel daemon` tracks agent sessions across terminal panes
2. `babel zellij-bridge` subscribes to the daemon's paint stream
3. Bridge pipes JSON state to this plugin via `zellij pipe --name babel`
4. Plugin renders the status bar with live activity indicators

## Setup

### Build

Requires `rust-wasm` package (Arch: `pacman -S rust-wasm`):

```sh
cargo build --target wasm32-wasi --release
```

### Zellij config

Add to your zellij layout or config:

```kdl
pane size=1 borderless=true {
    plugin location="file:/path/to/target/wasm32-wasi/release/babel-zellij.wasm"
}
```

### Start the bridge

```sh
babel zellij-bridge
```

Or add to your zellij layout as a background command pane.

## Sibling projects

- [babel](https://github.com/holo-q/babel) — The daemon and CLI
- [scrollparse](https://github.com/holo-q/scrollparse) — Terminal output parser for activity detection

Build expects sibling crates cloned alongside in the same directory (see babel's README for the full dependency tree).

## License

GPL-2.0-or-later
