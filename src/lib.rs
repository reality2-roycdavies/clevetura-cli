//! Library crate for clevetura-cli — talk to a Clevetura TouchOnKeys keyboard
//! over USB-HID and Bluetooth Low Energy.
//!
//! Frontends (the cosmic-clevetura applet, the kde-clevetura plasmoid via
//! the binary, or your own integration) use these modules to:
//!
//! - Discover keyboards (`hid::enumerate_devices`, `ble::scan_devices`)
//! - Read/write firmware settings via protobuf (`proto::*`, `keyboard::*`,
//!   `ble::BleConnection::*`)
//! - Persist user-side preferences (`config::Config`)
//!
//! See [`README.md`](https://github.com/reality2-roycdavies/clevetura-cli)
//! for a high-level overview and the binary CLI surface.

pub mod ble;
pub mod config;
pub mod hid;
pub mod keyboard;
pub mod proto;
pub mod slider_actions;
