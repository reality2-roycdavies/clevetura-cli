# clevetura-cli

Headless command-line tool for the [Clevetura CLVX-S](https://clevetura.com/) "TouchOnKeys" keyboard. Talks to the keyboard over USB-HID and Bluetooth Low Energy using its native protobuf-based protocol, and exposes everything as JSON-on-stdout for integration with desktop applets.

This is the shared backend for:

| Frontend | Desktop |
|----------|---------|
| **[cosmic-clevetura](https://github.com/reality2-roycdavies/cosmic-clevetura)** | COSMIC (Pop!_OS) |
| **[kde-clevetura](https://github.com/reality2-roycdavies/kde-clevetura)** | KDE Plasma 6 |

Each frontend is a thin UI that shells out to `clevetura-cli`. Hardware-comms code lives here, exactly once.

## What it does

- Detects connected Clevetura keyboards via USB-HID and Bluetooth LE
- Reads/writes firmware settings (sensitivity, gestures, key behaviour, sliders) via protobuf
- Persists user preferences in `~/.config/clevetura/config.json`
- Prints structured JSON for every primary command

## Installation

Requires a Rust toolchain and the system libraries used by `hidapi`/`btleplug`:

```bash
# Debian/Ubuntu/Pop!_OS
sudo apt install libudev-dev libdbus-1-dev pkg-config build-essential

# Fedora
sudo dnf install systemd-devel dbus-devel gcc

# Arch
sudo pacman -S systemd dbus base-devel
```

Then:

```bash
git clone https://github.com/reality2-roycdavies/clevetura-cli.git
cd clevetura-cli

./install.sh install              # builds release, installs to ~/.local/bin
sudo ./install.sh install-udev    # allows unprivileged HID access (recommended)
```

After the udev rule is installed, replug the keyboard.

## Usage

```bash
clevetura-cli help                       # list all commands
clevetura-cli status                     # JSON: connection, battery, FW, sensitivity
clevetura-cli detect                     # JSON: enumerate USB-HID Clevetura devices
clevetura-cli get-config                 # JSON: stored config
clevetura-cli describe-config            # JSON: settings UI schema (used by hubs)
clevetura-cli set-config <key> <jsonval> # update one config field; saves to disk
clevetura-cli reset-config               # reset config to defaults
clevetura-cli apply-config               # send full config to firmware via protobuf
clevetura-cli sync-from-firmware         # pull firmware settings into config
clevetura-cli get-firmware-settings      # raw firmware settings as JSON
clevetura-cli set-sensitivity <1-9>      # one-shot sensitivity change
clevetura-cli ai-on | ai-off | ai-state  # control AI touch processing
clevetura-cli factory-reset
clevetura-cli set-os-mode <0|1|2>        # 0=Win, 1=mac, 2=Linux
clevetura-cli ble-scan [--timeout=N]     # discover Clevetura BLE devices
clevetura-cli ble-info <address>         # device info via BLE
```

Debug commands print human-readable text instead of JSON:

```bash
clevetura-cli info     # detailed device dump
clevetura-cli probe    # enumerate HID feature reports
clevetura-cli watch    # watch report 0xDB for changes
```

## JSON conventions

Each primary subcommand prints exactly one JSON object on stdout, on a single line:

- **Success:** the relevant payload (e.g. `{"connected": true, "battery": 85, ...}`)
- **`set-*`/`reset-*`/`apply-*`:** `{"ok": true, "message": "..."}`
- **Failure:** `{"ok": false, "error": "..."}` with non-zero exit code

This makes the output trivially parseable from a shell, a Plasma plasmoid (`Plasma5Support.DataSource` + `JSON.parse`), or a libcosmic widget (`serde_json::from_str`).

## Protocol summary

Clevetura keyboards speak two layers:

- **Layer 1 (firmware commands):** request/response over HID report 0x21 / 0x22 — auth, battery, FW version, etc.
- **Layer 2 (app protocol):** protobuf messages base64-encoded over HID report 0x23 / 0x24 (USB) or over a single GATT characteristic with CRC32 framing (BLE).

See [`src/proto.rs`](src/proto.rs) for the full protobuf message definitions, reverse-engineered from the official TouchOnKeys app.

## Config file

`~/.config/clevetura/config.json` — JSON, pretty-printed. On first run, contents migrate from the older `~/.config/cosmic-clevetura/config.json` if present.

## License

MIT — see [LICENSE](LICENSE).

## Acknowledgments

- [Clevetura](https://clevetura.com/) for the keyboard hardware
- The TouchOnKeys app for documenting the protocol on the wire (reverse-engineered)
- [hidapi-rs](https://github.com/ruabmbua/hidapi-rs), [btleplug](https://github.com/deviceplug/btleplug), [prost](https://github.com/tokio-rs/prost) — Rust ecosystem dependencies
