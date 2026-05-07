# babel-zellij — Zellij plugin for babel agent status

target := "wasm32-wasip1"
wasm := "target/" + target + "/release/babel-zellij.wasm"

# Build the WASM plugin (release)
build:
    cargo build --target {{target}} --release

# Build debug
build-debug:
    cargo build --target {{target}}

# Install plugin to zellij's plugin dir
install: build
    mkdir -p ~/.config/zellij/plugins
    cp {{wasm}} ~/.config/zellij/plugins/babel-zellij.wasm
    @echo "Installed to ~/.config/zellij/plugins/babel-zellij.wasm"
    @echo "Use in layout: plugin location=\"file:~/.config/zellij/plugins/babel-zellij.wasm\""

# Check types
check:
    cargo check --target {{target}}

# Clean build artifacts
clean:
    cargo clean

# Print wasm size
size: build
    @ls -lh {{wasm}} | awk '{print $5, $9}'

# Create a GitHub release with the wasm binary attached
# Usage: just release v0.1.0
release version: build
    gh release create {{version}} {{wasm}} \
        --repo holo-q/babel-zellij \
        --title "{{version}}" \
        --notes "babel-zellij {{version}} — Zellij plugin for babel agent session status.\n\nAdd to your zellij config:\n\`\`\`kdl\npane size=1 borderless=true {\n    plugin location=\"https://github.com/holo-q/babel-zellij/releases/download/{{version}}/babel-zellij.wasm\"\n}\n\`\`\`"
