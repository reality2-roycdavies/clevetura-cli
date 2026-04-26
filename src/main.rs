//! clevetura-cli — desktop-neutral CLI to talk to a Clevetura TouchOnKeys keyboard.
//!
//! Every primary subcommand prints a single line of JSON on stdout. Errors go
//! to stderr; exit code 0 on success, non-zero on failure. Debug subcommands
//! (`info`, `probe`, `watch`) print human-readable text.

mod ble;
mod config;
mod hid;
mod keyboard;
mod proto;
mod slider_actions;

use std::process::ExitCode;

use serde_json::{json, Value};

use crate::config::Config;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();

    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    let rest: &[String] = if args.len() >= 2 { &args[2..] } else { &[] };

    match cmd {
        "status" => cmd_status(),
        "detect" => cmd_detect(),
        "info" => { keyboard::print_device_info(); ExitCode::SUCCESS }
        "probe" => { keyboard::probe_reports(); ExitCode::SUCCESS }
        "watch" => { keyboard::watch_reports(); ExitCode::SUCCESS }
        "get-firmware-settings" => cmd_get_firmware_settings(),
        "get-config" => cmd_get_config(),
        "describe-config" => cmd_describe_config(),
        "set-config" => cmd_set_config(rest),
        "reset-config" => cmd_reset_config(),
        "apply-config" => cmd_apply_config(),
        "sync-from-firmware" => cmd_sync_from_firmware(),
        "set-sensitivity" => cmd_set_sensitivity(rest),
        "ai-on" => cmd_control_ai(1),
        "ai-off" => cmd_control_ai(0),
        "ai-state" => cmd_ai_state(),
        "factory-reset" => cmd_factory_reset(),
        "set-os-mode" => cmd_set_os_mode(rest),
        "ble-scan" => cmd_ble_scan(rest),
        "ble-info" => cmd_ble_info(rest),
        "version" | "--version" | "-v" => {
            print_json(&json!({"version": env!("CARGO_PKG_VERSION")}));
            ExitCode::SUCCESS
        }
        "help" | "--help" | "-h" => { print_help(&args[0]); ExitCode::SUCCESS }
        other => {
            print_err(&format!("Unknown subcommand: {other}"));
            ExitCode::FAILURE
        }
    }
}

// ── Output helpers ──────────────────────────────────────────────────────────

fn print_json(v: &Value) {
    // Single-line JSON so plasmoid line-parsers stay simple
    println!("{}", v);
}

fn print_ok(message: &str) -> ExitCode {
    print_json(&json!({"ok": true, "message": message}));
    ExitCode::SUCCESS
}

fn print_err(message: &str) -> ExitCode {
    print_json(&json!({"ok": false, "error": message}));
    ExitCode::FAILURE
}

// ── status ──────────────────────────────────────────────────────────────────

fn cmd_status() -> ExitCode {
    let conn = match keyboard::KeyboardConnection::open() {
        Ok(c) => c,
        Err(e) => {
            print_json(&json!({
                "connected": false,
                "transport": null,
                "error": e,
            }));
            return ExitCode::SUCCESS;
        }
    };
    let _ = conn.authorize();

    let info = conn.device_info().clone();
    let battery = conn.get_battery_level().ok();
    let fw = conn.get_firmware_version().ok();
    let proto_v = conn.get_protocol_version().ok();
    let serial = conn.get_serial_number().ok();

    // Try to read firmware settings to surface current sensitivity etc.
    let settings = proto::get_settings(conn.device()).ok();
    let sensitivity = settings
        .as_ref()
        .and_then(|s| s.global.as_ref())
        .and_then(|g| g.current_ai_level);

    let body = json!({
        "connected": true,
        "transport": "usb",
        "interface": info.interface_number,
        "vid": format!("{:04x}", info.vendor_id),
        "pid": format!("{:04x}", info.product_id),
        "product": info.product_name,
        "serial": serial.unwrap_or(info.serial),
        "battery": battery,
        "fw_version": fw,
        "protocol_version": proto_v,
        "sensitivity": sensitivity,
    });
    print_json(&body);
    ExitCode::SUCCESS
}

// ── detect ──────────────────────────────────────────────────────────────────

fn cmd_detect() -> ExitCode {
    match hid::enumerate_devices() {
        Ok(devices) => {
            let arr: Vec<Value> = devices
                .iter()
                .map(|d| {
                    json!({
                        "product": d.product_name,
                        "vid": format!("{:04x}", d.vendor_id),
                        "pid": format!("{:04x}", d.product_id),
                        "serial": d.serial,
                        "interface": d.interface_number,
                        "path": d.path,
                        "usage_page": format!("{:04x}", d.usage_page),
                        "usage": format!("{:04x}", d.usage),
                    })
                })
                .collect();
            print_json(&json!({"devices": arr}));
            ExitCode::SUCCESS
        }
        Err(e) => print_err(&e),
    }
}

// ── get-firmware-settings ───────────────────────────────────────────────────

fn cmd_get_firmware_settings() -> ExitCode {
    let conn = match keyboard::KeyboardConnection::open() {
        Ok(c) => c,
        Err(e) => return print_err(&e),
    };
    let _ = conn.authorize();

    match proto::get_settings(conn.device()) {
        Ok(s) => {
            let g = s.global.as_ref();
            print_json(&json!({
                "ok": true,
                "global": g.map(|g| json!({
                    "current_ai_level": g.current_ai_level,
                    "tap1f_enable": g.tap1f_enable,
                    "tap2f_enable": g.tap2f_enable,
                    "hold_enable": g.hold_enable,
                    "swap_click_buttons": g.swap_click_buttons,
                    "fn_lock": g.fn_lock,
                    "dominant_hand": g.dominant_hand,
                    "swap_fn_ctrl": g.swap_fn_ctrl,
                    "auto_brightness_enable": g.auto_brightness_enable,
                    "battery_saving_mode_enable": g.battery_saving_mode_enable,
                    "newbie_mode_enable": g.newbie_mode_enable,
                    "key_suppressor_enable": g.key_suppressor_enable,
                    "hold_delay_on_border_enable": g.hold_delay_on_border_enable,
                    "touch_activation_after_lift_off": g.touch_activation_after_lift_off,
                })),
                "profile_id": s.global_profile.as_ref().map(|p| p.id),
            }));
            ExitCode::SUCCESS
        }
        Err(e) => print_err(&e),
    }
}

// ── get-config ──────────────────────────────────────────────────────────────

fn cmd_get_config() -> ExitCode {
    let cfg = Config::load();
    match serde_json::to_value(&cfg) {
        Ok(v) => { print_json(&v); ExitCode::SUCCESS }
        Err(e) => print_err(&format!("Serialize failed: {e}")),
    }
}

// ── describe-config ─────────────────────────────────────────────────────────

fn cmd_describe_config() -> ExitCode {
    let config = Config::load();

    let slider_options = json!([
        {"value": "brightness", "label": "Backlight Brightness"},
        {"value": "volume", "label": "System Volume"},
        {"value": "media_scrub", "label": "Media Scrub"},
        {"value": "zoom", "label": "Zoom Level"},
        {"value": "scroll_speed", "label": "Scroll Speed"}
    ]);

    let schema = json!({
        "title": "Clevetura TouchOnKeys Settings",
        "description": "Configure your Clevetura keyboard's touch sensitivity, gestures, and hardware settings.",
        "sections": [
            {
                "title": "Touch Sensitivity",
                "items": [
                    {"type":"number","key":"sensitivity","label":"Sensitivity Level (1-9)","value":config.sensitivity,"min":1,"max":9}
                ]
            },
            {
                "title": "Touch Behaviour",
                "items": [
                    {"type":"toggle","key":"tap_1f","label":"Single-finger tap","value":config.tap_1f},
                    {"type":"toggle","key":"tap_2f","label":"Two-finger tap (right-click)","value":config.tap_2f},
                    {"type":"toggle","key":"hold_gesture","label":"Hold gesture","value":config.hold_gesture},
                    {"type":"toggle","key":"swap_clicks","label":"Swap click buttons (left/right)","value":config.swap_clicks},
                    {"type":"toggle","key":"touch_after_liftoff","label":"Touch activation after lift-off","value":config.touch_after_liftoff},
                    {"type":"toggle","key":"newbie_mode","label":"Newbie mode (simplified touch)","value":config.newbie_mode},
                    {"type":"toggle","key":"key_suppressor","label":"Key suppressor (prevent accidental keys during touch)","value":config.key_suppressor},
                    {"type":"toggle","key":"hold_delay_on_border","label":"Hold delay on border","value":config.hold_delay_on_border}
                ]
            },
            {
                "title": "Keyboard",
                "items": [
                    {"type":"toggle","key":"fn_lock","label":"Fn Lock (Fn key always active)","value":config.fn_lock},
                    {"type":"toggle","key":"left_handed","label":"Left-handed mode","value":config.left_handed},
                    {"type":"toggle","key":"swap_fn_ctrl","label":"Swap Fn and Ctrl keys","value":config.swap_fn_ctrl}
                ]
            },
            {
                "title": "Hardware",
                "items": [
                    {"type":"toggle","key":"auto_brightness","label":"Auto brightness","value":config.auto_brightness},
                    {"type":"toggle","key":"battery_saving","label":"Battery saving mode","value":config.battery_saving}
                ]
            },
            {
                "title": "Touch Sliders",
                "items": [
                    {"type":"select","key":"left_slider","label":"Left Slider (F2-F6)","value":slider_action_to_cli(&config.left_slider),"options":slider_options},
                    {"type":"select","key":"right_slider","label":"Right Slider (F7-F11)","value":slider_action_to_cli(&config.right_slider),"options":slider_options}
                ]
            },
            {
                "title": "Profiles",
                "items": [
                    {"type":"toggle","key":"profiles_enabled","label":"Enable per-app profiles","value":config.profiles_enabled}
                ]
            }
        ],
        "actions": [
            {"id":"reset","label":"Reset to Defaults","style":"destructive"}
        ]
    });

    print_json(&schema);
    ExitCode::SUCCESS
}

// ── set-config ──────────────────────────────────────────────────────────────

fn cmd_set_config(args: &[String]) -> ExitCode {
    if args.len() < 2 {
        return print_err("Usage: clevetura-cli set-config <key> <json_value>");
    }
    let key = &args[0];
    let value = &args[1];

    let mut config = Config::load();

    let result: Result<&str, String> = match key.as_str() {
        "sensitivity" => match serde_json::from_str::<u8>(value) {
            Ok(level) if (1..=9).contains(&level) => { config.sensitivity = level; Ok("Updated sensitivity") }
            Ok(_) => Err("Sensitivity must be 1-9".into()),
            Err(e) => Err(format!("Invalid number: {e}")),
        },
        "left_slider" => cli_to_slider_action(value).map(|a| { config.left_slider = a; "Updated left slider" }),
        "right_slider" => cli_to_slider_action(value).map(|a| { config.right_slider = a; "Updated right slider" }),
        "profiles_enabled" => set_bool_field(&mut config.profiles_enabled, value),
        "tap_1f" => set_bool_field(&mut config.tap_1f, value),
        "tap_2f" => set_bool_field(&mut config.tap_2f, value),
        "hold_gesture" => set_bool_field(&mut config.hold_gesture, value),
        "swap_clicks" => set_bool_field(&mut config.swap_clicks, value),
        "touch_after_liftoff" => set_bool_field(&mut config.touch_after_liftoff, value),
        "newbie_mode" => set_bool_field(&mut config.newbie_mode, value),
        "key_suppressor" => set_bool_field(&mut config.key_suppressor, value),
        "hold_delay_on_border" => set_bool_field(&mut config.hold_delay_on_border, value),
        "fn_lock" => set_bool_field(&mut config.fn_lock, value),
        "left_handed" => set_bool_field(&mut config.left_handed, value),
        "swap_fn_ctrl" => set_bool_field(&mut config.swap_fn_ctrl, value),
        "auto_brightness" => set_bool_field(&mut config.auto_brightness, value),
        "battery_saving" => set_bool_field(&mut config.battery_saving, value),
        _ => Err(format!("Unknown key: {key}")),
    };

    match result {
        Ok(msg) => match config.save() {
            Ok(()) => print_ok(msg),
            Err(e) => print_err(&format!("Save failed: {e}")),
        },
        Err(e) => print_err(&e),
    }
}

fn set_bool_field<'a>(field: &mut bool, value: &str) -> Result<&'a str, String> {
    match serde_json::from_str::<bool>(value) {
        Ok(v) => { *field = v; Ok("Updated") }
        Err(e) => Err(format!("Invalid boolean: {e}")),
    }
}

fn slider_action_to_cli(action: &config::SliderAction) -> &'static str {
    use config::SliderAction;
    match action {
        SliderAction::Brightness => "brightness",
        SliderAction::Volume => "volume",
        SliderAction::MediaScrub => "media_scrub",
        SliderAction::ZoomLevel => "zoom",
        SliderAction::ScrollSpeed => "scroll_speed",
        SliderAction::Custom(_) => "brightness",
    }
}

fn cli_to_slider_action(value: &str) -> Result<config::SliderAction, String> {
    use config::SliderAction;
    let s: String = serde_json::from_str(value).map_err(|e| format!("Invalid string: {e}"))?;
    match s.as_str() {
        "brightness" => Ok(SliderAction::Brightness),
        "volume" => Ok(SliderAction::Volume),
        "media_scrub" => Ok(SliderAction::MediaScrub),
        "zoom" => Ok(SliderAction::ZoomLevel),
        "scroll_speed" => Ok(SliderAction::ScrollSpeed),
        _ => Err(format!("Unknown slider action: {s}")),
    }
}

// ── reset-config ────────────────────────────────────────────────────────────

fn cmd_reset_config() -> ExitCode {
    let cfg = Config::default();
    match cfg.save() {
        Ok(()) => print_ok("Reset to defaults"),
        Err(e) => print_err(&format!("Reset failed: {e}")),
    }
}

// ── apply-config ────────────────────────────────────────────────────────────

fn cmd_apply_config() -> ExitCode {
    let cfg = Config::load();
    let conn = match keyboard::KeyboardConnection::open() {
        Ok(c) => c,
        Err(e) => return print_err(&e),
    };
    let _ = conn.authorize();
    let settings = proto::AppSettings {
        global: Some(cfg.to_global_settings()),
        global_profile: None,
        counter: None,
    };
    match proto::set_settings(conn.device(), settings) {
        Ok(()) => print_ok("Config applied to keyboard"),
        Err(e) => print_err(&e),
    }
}

// ── sync-from-firmware ──────────────────────────────────────────────────────

fn cmd_sync_from_firmware() -> ExitCode {
    let conn = match keyboard::KeyboardConnection::open() {
        Ok(c) => c,
        Err(e) => return print_err(&e),
    };
    let _ = conn.authorize();

    let settings = match proto::get_settings(conn.device()) {
        Ok(s) => s,
        Err(e) => return print_err(&e),
    };

    let mut cfg = Config::load();
    if let Some(g) = settings.global.as_ref() {
        cfg.update_from_firmware(g);
    }
    match cfg.save() {
        Ok(()) => print_ok("Synced firmware settings to config"),
        Err(e) => print_err(&format!("Save failed: {e}")),
    }
}

// ── set-sensitivity ─────────────────────────────────────────────────────────

fn cmd_set_sensitivity(args: &[String]) -> ExitCode {
    if args.is_empty() {
        return print_err("Usage: clevetura-cli set-sensitivity <1-9>");
    }
    let level: u32 = match args[0].parse() {
        Ok(n) => n,
        Err(_) => return print_err("Sensitivity must be a number"),
    };
    if !(1..=9).contains(&level) {
        return print_err("Sensitivity must be 1-9");
    }

    let conn = match keyboard::KeyboardConnection::open() {
        Ok(c) => c,
        Err(e) => return print_err(&e),
    };
    let _ = conn.authorize();

    let settings = proto::AppSettings {
        global: Some(proto::GlobalSettings {
            current_ai_level: Some(level),
            ..Default::default()
        }),
        global_profile: None,
        counter: None,
    };

    match proto::set_settings(conn.device(), settings) {
        Ok(()) => {
            // Persist to config too so the next status call reflects it
            let mut cfg = Config::load();
            cfg.sensitivity = level as u8;
            let _ = cfg.save();
            print_ok(&format!("Sensitivity set to {level}"))
        }
        Err(e) => print_err(&e),
    }
}

// ── ai-on / ai-off / ai-state ───────────────────────────────────────────────

fn cmd_control_ai(mode: i32) -> ExitCode {
    let conn = match keyboard::KeyboardConnection::open() {
        Ok(c) => c,
        Err(e) => return print_err(&e),
    };
    let _ = conn.authorize();

    let request = proto::Request {
        r#type: proto::RequestType::ControlAi as i32,
        control_ai: Some(proto::ControlAiRequest { mode }),
        ..Default::default()
    };
    match proto::send_proto_request(conn.device(), &request) {
        Ok(_) => print_ok(if mode == 1 { "AI on" } else { "AI off" }),
        Err(e) => print_err(&e),
    }
}

fn cmd_ai_state() -> ExitCode {
    let conn = match keyboard::KeyboardConnection::open() {
        Ok(c) => c,
        Err(e) => return print_err(&e),
    };
    let _ = conn.authorize();

    let request = proto::Request {
        r#type: proto::RequestType::GetAiState as i32,
        get_ai_state: Some(proto::GetAiStateRequest {}),
        ..Default::default()
    };
    match proto::send_proto_request(conn.device(), &request) {
        Ok(resp) => {
            let ai = resp.get_ai_state.unwrap_or_default();
            print_json(&json!({"mode": ai.mode, "active": ai.active}));
            ExitCode::SUCCESS
        }
        Err(e) => print_err(&e),
    }
}

// ── factory-reset ───────────────────────────────────────────────────────────

fn cmd_factory_reset() -> ExitCode {
    let conn = match keyboard::KeyboardConnection::open() {
        Ok(c) => c,
        Err(e) => return print_err(&e),
    };
    let _ = conn.authorize();
    let request = proto::Request {
        r#type: proto::RequestType::PerformFullReset as i32,
        ..Default::default()
    };
    match proto::send_proto_request(conn.device(), &request) {
        Ok(_) => print_ok("Factory reset sent"),
        Err(e) => print_err(&e),
    }
}

// ── set-os-mode ─────────────────────────────────────────────────────────────

fn cmd_set_os_mode(args: &[String]) -> ExitCode {
    if args.is_empty() {
        return print_err("Usage: clevetura-cli set-os-mode <0=win|1=mac|2=linux>");
    }
    let mode: i32 = match args[0].parse() {
        Ok(n) => n,
        Err(_) => return print_err("OS mode must be a number"),
    };
    if !(0..=2).contains(&mode) {
        return print_err("OS mode must be 0, 1, or 2");
    }
    let conn = match keyboard::KeyboardConnection::open() {
        Ok(c) => c,
        Err(e) => return print_err(&e),
    };
    let _ = conn.authorize();
    match proto::set_os_mode(conn.device(), mode) {
        Ok(()) => print_ok(&format!("OS mode set to {}", ["Windows","macOS","Linux"][mode as usize])),
        Err(e) => print_err(&e),
    }
}

// ── ble-scan ────────────────────────────────────────────────────────────────

fn cmd_ble_scan(args: &[String]) -> ExitCode {
    let timeout_secs: u64 = args
        .iter()
        .find_map(|a| a.strip_prefix("--timeout=").and_then(|v| v.parse().ok()))
        .unwrap_or(5);

    let rt = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => return print_err(&format!("Tokio runtime: {e}")),
    };

    let result = rt.block_on(async {
        ble::scan_devices(std::time::Duration::from_secs(timeout_secs)).await
    });

    match result {
        Ok(devs) => {
            let arr: Vec<Value> = devs
                .iter()
                .map(|d| json!({"name": d.name, "address": d.address}))
                .collect();
            print_json(&json!({"devices": arr}));
            ExitCode::SUCCESS
        }
        Err(e) => print_err(&e),
    }
}

// ── ble-info ────────────────────────────────────────────────────────────────

fn cmd_ble_info(args: &[String]) -> ExitCode {
    if args.is_empty() {
        return print_err("Usage: clevetura-cli ble-info <address>");
    }
    let address = args[0].clone();
    let rt = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => return print_err(&format!("Tokio runtime: {e}")),
    };

    rt.block_on(async {
        match ble::BleConnection::connect_by_address(&address).await {
            Ok(conn) => {
                let _ = conn.authorize().await;
                let battery = conn.heartbeat(0).await.ok().flatten();
                let settings = conn.get_settings().await.ok();
                let level = settings
                    .as_ref()
                    .and_then(|s| s.global.as_ref())
                    .and_then(|g| g.current_ai_level);
                let _ = conn.disconnect().await;
                print_json(&json!({
                    "connected": true,
                    "transport": "ble",
                    "address": address,
                    "battery": battery.as_ref().map(|b| b.level),
                    "charging": battery.as_ref().map(|b| b.charging),
                    "sensitivity": level,
                }));
                ExitCode::SUCCESS
            }
            Err(e) => print_err(&e),
        }
    })
}

// ── help ────────────────────────────────────────────────────────────────────

fn print_help(prog: &str) {
    eprintln!("clevetura-cli {} — talk to a Clevetura TouchOnKeys keyboard", env!("CARGO_PKG_VERSION"));
    eprintln!();
    eprintln!("Usage: {prog} <command> [args]");
    eprintln!();
    eprintln!("Primary commands (output JSON on stdout):");
    eprintln!("  status                       Connection state, battery, FW, sensitivity");
    eprintln!("  detect                       Enumerate USB-HID Clevetura devices");
    eprintln!("  get-config                   Print stored config (~/.config/clevetura/config.json)");
    eprintln!("  describe-config              Print settings UI schema for config editors");
    eprintln!("  set-config <key> <json_val>  Update one config field; saves to disk");
    eprintln!("  reset-config                 Reset config to defaults");
    eprintln!("  apply-config                 Send full config to firmware via protobuf");
    eprintln!("  sync-from-firmware           Read firmware settings into config");
    eprintln!("  get-firmware-settings        Read firmware settings, print JSON");
    eprintln!("  set-sensitivity <1-9>        Set AI touch sensitivity (immediate)");
    eprintln!("  ai-on | ai-off | ai-state    Control AI touch processing");
    eprintln!("  factory-reset                Trigger keyboard factory reset");
    eprintln!("  set-os-mode <0|1|2>          Set OS mode (Windows/macOS/Linux)");
    eprintln!("  ble-scan [--timeout=N]       Scan for Clevetura BLE devices (N secs)");
    eprintln!("  ble-info <address>           Connect via BLE, fetch device info");
    eprintln!("  version                      Print CLI version");
    eprintln!();
    eprintln!("Debug commands (human-readable):");
    eprintln!("  info                         Detailed device info dump");
    eprintln!("  probe                        Enumerate HID feature reports");
    eprintln!("  watch                        Watch report 0xDB for changes");
    eprintln!();
    eprintln!("Config file: ~/.config/clevetura/config.json");
}
