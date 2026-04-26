#!/bin/bash
# clevetura-cli installer.
# Builds the binary and installs it to ~/.local/bin (or $PREFIX/bin if PREFIX is set).
# Optionally installs the udev rule and three SVG icons system-wide.

set -euo pipefail

PREFIX="${PREFIX:-$HOME/.local}"
BIN_DIR="$PREFIX/bin"
ICON_DIR="$PREFIX/share/icons/hicolor/symbolic/apps"
UDEV_DIR="/etc/udev/rules.d"

REPO_DIR="$(dirname "$(realpath "$0")")"

case "${1:-install}" in
    install)
        echo "Building clevetura-cli (release)..."
        ( cd "$REPO_DIR" && cargo build --release )

        echo "Installing binary to $BIN_DIR/clevetura-cli ..."
        mkdir -p "$BIN_DIR"
        install -m755 "$REPO_DIR/target/release/clevetura-cli" "$BIN_DIR/clevetura-cli"

        echo "Installing icons to $ICON_DIR ..."
        mkdir -p "$ICON_DIR"
        install -m644 "$REPO_DIR/resources/clevetura-symbolic.svg" "$ICON_DIR/clevetura-symbolic.svg"
        install -m644 "$REPO_DIR/resources/clevetura-connected-symbolic.svg" "$ICON_DIR/clevetura-connected-symbolic.svg"
        install -m644 "$REPO_DIR/resources/clevetura-disconnected-symbolic.svg" "$ICON_DIR/clevetura-disconnected-symbolic.svg"

        echo "Done."
        echo
        echo "Run 'clevetura-cli help' to see commands."
        echo "Run 'sudo $0 install-udev' to install the udev rule (recommended)."
        ;;

    install-udev)
        if [ "$(id -u)" != "0" ]; then
            echo "install-udev requires sudo. Try: sudo $0 install-udev"
            exit 1
        fi
        echo "Installing udev rule to $UDEV_DIR/99-clevetura.rules ..."
        install -m644 "$REPO_DIR/resources/99-clevetura.rules" "$UDEV_DIR/99-clevetura.rules"
        udevadm control --reload-rules
        udevadm trigger
        echo "Done. Replug the keyboard for the rule to take effect."
        ;;

    uninstall-udev)
        if [ "$(id -u)" != "0" ]; then
            echo "uninstall-udev requires sudo. Try: sudo $0 uninstall-udev"
            exit 1
        fi
        rm -f "$UDEV_DIR/99-clevetura.rules"
        udevadm control --reload-rules
        echo "Done."
        ;;

    uninstall)
        rm -f "$BIN_DIR/clevetura-cli"
        rm -f "$ICON_DIR/clevetura-symbolic.svg"
        rm -f "$ICON_DIR/clevetura-connected-symbolic.svg"
        rm -f "$ICON_DIR/clevetura-disconnected-symbolic.svg"
        echo "Removed clevetura-cli binary and icons. Udev rule (if installed) left untouched — see uninstall-udev."
        ;;

    *)
        echo "Usage: $0 {install|uninstall|install-udev|uninstall-udev}"
        exit 1
        ;;
esac
