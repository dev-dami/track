#!/bin/bash
set -e

# Track Language Installer
# Installs track and track-lsp binaries to /usr/local/bin

INSTALL_DIR="/usr/local/bin"
BINARIES=("track" "track-lsp")

echo "Installing Track language..."

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    echo "Error: cargo not found. Install Rust first: https://rustup.rs"
    exit 1
fi

# Build release binaries
echo "Building release binaries..."
cargo build --release

# Check if build succeeded
for binary in "${BINARIES[@]}"; do
    BINARY_PATH="target/release/$binary"
    if [ ! -f "$BINARY_PATH" ]; then
        echo "Error: Build failed. Binary not found at $BINARY_PATH"
        exit 1
    fi
done

# Create install directory if it doesn't exist
if [ ! -d "$INSTALL_DIR" ]; then
    echo "Creating $INSTALL_DIR..."
    sudo mkdir -p "$INSTALL_DIR"
fi

# Copy binaries
for binary in "${BINARIES[@]}"; do
    echo "Installing $binary to $INSTALL_DIR/$binary..."
    sudo cp "target/release/$binary" "$INSTALL_DIR/$binary"
    sudo chmod +x "$INSTALL_DIR/$binary"
done

# Verify installation
if command -v "track" &> /dev/null; then
    echo "Track installed successfully!"
    echo "Run 'track --help' to get started."
    echo "Run 'track-lsp' to start the language server."
else
    echo "Warning: track installed but not in PATH. You may need to add $INSTALL_DIR to your PATH."
fi
