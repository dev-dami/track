#!/bin/bash
set -e

# Track Language Installer
# Installs the track binary to /usr/local/bin

INSTALL_DIR="/usr/local/bin"
BINARY_NAME="track"

echo "Installing Track language..."

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    echo "Error: cargo not found. Install Rust first: https://rustup.rs"
    exit 1
fi

# Build release binary
echo "Building release binary..."
cargo build --release

# Check if build succeeded
BINARY_PATH="target/release/$BINARY_NAME"
if [ ! -f "$BINARY_PATH" ]; then
    echo "Error: Build failed. Binary not found at $BINARY_PATH"
    exit 1
fi

# Create install directory if it doesn't exist
if [ ! -d "$INSTALL_DIR" ]; then
    echo "Creating $INSTALL_DIR..."
    sudo mkdir -p "$INSTALL_DIR"
fi

# Copy binary
echo "Installing to $INSTALL_DIR/$BINARY_NAME..."
sudo cp "$BINARY_PATH" "$INSTALL_DIR/$BINARY_NAME"
sudo chmod +x "$INSTALL_DIR/$BINARY_NAME"

# Verify installation
if command -v "$BINARY_NAME" &> /dev/null; then
    echo "Track installed successfully!"
    echo "Run 'track --help' to get started."
else
    echo "Warning: track installed but not in PATH. You may need to add $INSTALL_DIR to your PATH."
fi
