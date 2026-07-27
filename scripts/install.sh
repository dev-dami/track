#!/bin/bash
set -e

# Track Language & Yard Toolchain Installer
# Clones the latest Track repo to a temporary directory, builds release binaries,
# installs track, yard, and track-lsp, sets up PATH, and cleans up temporary files.

echo "Installing Track language & Yard package manager..."

# Check if git is available
if ! command -v git &> /dev/null; then
    echo "Error: git is not installed."
    exit 1
fi

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    echo "Error: cargo not found. Please install Rust first: https://rustup.rs"
    exit 1
fi

# Create a temporary working directory and set cleanup trap
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

echo "Cloning Track repository..."
git clone --depth 1 https://github.com/dev-dami/track.git "$TMP_DIR/track"

cd "$TMP_DIR/track"

echo "Building release binaries (track, yard, track-lsp)..."
cargo build --release --bins

INSTALL_DIR="${TRACK_INSTALL_DIR:-$HOME/.track/bin}"
mkdir -p "$INSTALL_DIR"

BINARIES=("track" "yard" "track-lsp")

for binary in "${BINARIES[@]}"; do
    BINARY_PATH="target/release/$binary"
    if [ ! -f "$BINARY_PATH" ]; then
        echo "Error: Build failed. Binary not found at $BINARY_PATH"
        exit 1
    fi
    cp "$BINARY_PATH" "$INSTALL_DIR/$binary"
    chmod +x "$INSTALL_DIR/$binary"
    echo "Installed $binary to $INSTALL_DIR/$binary"
done

# PATH Configuration & Shell Setup
SHELL_NAME=$(basename "${SHELL:-bash}")
PROFILE_FILE=""

case "$SHELL_NAME" in
    bash)
        if [ -f "$HOME/.bashrc" ]; then
            PROFILE_FILE="$HOME/.bashrc"
        elif [ -f "$HOME/.bash_profile" ]; then
            PROFILE_FILE="$HOME/.bash_profile"
        fi
        ;;
    zsh)
        PROFILE_FILE="$HOME/.zshrc"
        ;;
    fish)
        PROFILE_FILE="$HOME/.config/fish/config.fish"
        ;;
    *)
        PROFILE_FILE="$HOME/.profile"
        ;;
esac

PATH_LINE="export PATH=\"$INSTALL_DIR:\$PATH\""

if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo ""
    echo "Adding $INSTALL_DIR to PATH in $PROFILE_FILE..."
    if [ -n "$PROFILE_FILE" ] && [ -f "$PROFILE_FILE" ]; then
        if ! grep -q "$INSTALL_DIR" "$PROFILE_FILE"; then
            echo "" >> "$PROFILE_FILE"
            echo "# Track Language" >> "$PROFILE_FILE"
            echo "$PATH_LINE" >> "$PROFILE_FILE"
            echo "Added $INSTALL_DIR to $PROFILE_FILE"
        fi
    fi
    export PATH="$INSTALL_DIR:$PATH"
fi

echo ""
echo "Track installed successfully!"
echo "Version: $($INSTALL_DIR/track --version)"
echo "Yard:    $($INSTALL_DIR/yard --version)"
echo ""
echo "To get started:"
echo "  track --help"
echo "  yard init my_app"
echo "  yard run"
echo ""
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo "Run 'source $PROFILE_FILE' or restart your terminal to update PATH."
fi
