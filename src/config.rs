use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Action that a touch slider can perform.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SliderAction {
    Brightness,
    Volume,
    MediaScrub,
    ZoomLevel,
    ScrollSpeed,
    Custom(String),
}

impl SliderAction {
    pub fn label(&self) -> &str {
        match self {
            SliderAction::Brightness => "Backlight Brightness",
            SliderAction::Volume => "System Volume",
            SliderAction::MediaScrub => "Media Scrub",
            SliderAction::ZoomLevel => "Zoom Level",
            SliderAction::ScrollSpeed => "Scroll Speed",
            SliderAction::Custom(_) => "Custom",
        }
    }

    pub fn all_standard() -> &'static [SliderAction] {
        &[
            SliderAction::Brightness,
            SliderAction::Volume,
            SliderAction::MediaScrub,
            SliderAction::ZoomLevel,
            SliderAction::ScrollSpeed,
        ]
    }
}

impl std::fmt::Display for SliderAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Per-application profile settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppProfile {
    pub name: String,
    pub app_id: String,
    pub sensitivity: u8,
    pub left_slider: SliderAction,
    pub right_slider: SliderAction,
}

impl Default for AppProfile {
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
            app_id: String::new(),
            sensitivity: 5,
            left_slider: SliderAction::Brightness,
            right_slider: SliderAction::Volume,
        }
    }
}

/// Top-level configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub sensitivity: u8,
    pub left_slider: SliderAction,
    pub right_slider: SliderAction,
    pub profiles: Vec<AppProfile>,
    pub profiles_enabled: bool,
    pub ble_address: Option<String>,

    // ── Firmware settings ──
    #[serde(default = "default_true")]
    pub tap_1f: bool,
    #[serde(default = "default_true")]
    pub tap_2f: bool,
    #[serde(default = "default_true")]
    pub hold_gesture: bool,
    #[serde(default)]
    pub swap_clicks: bool,
    #[serde(default)]
    pub fn_lock: bool,
    #[serde(default)]
    pub left_handed: bool,
    #[serde(default)]
    pub swap_fn_ctrl: bool,
    #[serde(default = "default_true")]
    pub auto_brightness: bool,
    #[serde(default)]
    pub battery_saving: bool,
    #[serde(default)]
    pub newbie_mode: bool,
    #[serde(default)]
    pub key_suppressor: bool,
    #[serde(default)]
    pub hold_delay_on_border: bool,
    #[serde(default)]
    pub touch_after_liftoff: bool,
}

fn default_true() -> bool { true }

impl Default for Config {
    fn default() -> Self {
        Self {
            sensitivity: 5,
            left_slider: SliderAction::Brightness,
            right_slider: SliderAction::Volume,
            profiles: Vec::new(),
            profiles_enabled: false,
            ble_address: None,
            tap_1f: true,
            tap_2f: true,
            hold_gesture: true,
            swap_clicks: false,
            fn_lock: false,
            left_handed: false,
            swap_fn_ctrl: false,
            auto_brightness: true,
            battery_saving: false,
            newbie_mode: false,
            key_suppressor: false,
            hold_delay_on_border: false,
            touch_after_liftoff: false,
        }
    }
}

impl Config {
    pub fn to_global_settings(&self) -> crate::proto::GlobalSettings {
        crate::proto::GlobalSettings {
            current_ai_level: Some(self.sensitivity as u32),
            tap1f_enable: Some(self.tap_1f),
            tap2f_enable: Some(self.tap_2f),
            hold_enable: Some(self.hold_gesture),
            swap_click_buttons: Some(self.swap_clicks),
            fn_lock: Some(self.fn_lock),
            dominant_hand: Some(if self.left_handed { 1 } else { 0 }),
            swap_fn_ctrl: Some(self.swap_fn_ctrl),
            auto_brightness_enable: Some(self.auto_brightness),
            battery_saving_mode_enable: Some(self.battery_saving),
            newbie_mode_enable: Some(self.newbie_mode),
            key_suppressor_enable: Some(self.key_suppressor),
            hold_delay_on_border_enable: Some(self.hold_delay_on_border),
            touch_activation_after_lift_off: Some(self.touch_after_liftoff),
        }
    }

    pub fn update_from_firmware(&mut self, global: &crate::proto::GlobalSettings) {
        if let Some(v) = global.current_ai_level { self.sensitivity = v.clamp(1, 9) as u8; }
        if let Some(v) = global.tap1f_enable { self.tap_1f = v; }
        if let Some(v) = global.tap2f_enable { self.tap_2f = v; }
        if let Some(v) = global.hold_enable { self.hold_gesture = v; }
        if let Some(v) = global.swap_click_buttons { self.swap_clicks = v; }
        if let Some(v) = global.fn_lock { self.fn_lock = v; }
        if let Some(v) = global.dominant_hand { self.left_handed = v == 1; }
        if let Some(v) = global.swap_fn_ctrl { self.swap_fn_ctrl = v; }
        if let Some(v) = global.auto_brightness_enable { self.auto_brightness = v; }
        if let Some(v) = global.battery_saving_mode_enable { self.battery_saving = v; }
        if let Some(v) = global.newbie_mode_enable { self.newbie_mode = v; }
        if let Some(v) = global.key_suppressor_enable { self.key_suppressor = v; }
        if let Some(v) = global.hold_delay_on_border_enable { self.hold_delay_on_border = v; }
        if let Some(v) = global.touch_activation_after_lift_off { self.touch_after_liftoff = v; }
    }

    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("clevetura").join("config.json"))
    }

    /// Legacy path used by cosmic-clevetura before the CLI split.
    fn legacy_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("cosmic-clevetura").join("config.json"))
    }

    /// Load config, migrating from the legacy path on first run if needed.
    pub fn load() -> Self {
        if let Some(p) = Self::config_path() {
            if let Ok(content) = std::fs::read_to_string(&p) {
                if let Ok(cfg) = serde_json::from_str(&content) {
                    return cfg;
                }
            }
        }
        // Try legacy path; if present, parse and migrate
        if let Some(legacy) = Self::legacy_config_path() {
            if let Ok(content) = std::fs::read_to_string(&legacy) {
                if let Ok(cfg) = serde_json::from_str::<Self>(&content) {
                    let _ = cfg.save();
                    return cfg;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path().ok_or("Could not determine config path")?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config dir: {e}"))?;
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {e}"))?;

        std::fs::write(path, content).map_err(|e| format!("Failed to write config: {e}"))?;

        Ok(())
    }

    pub fn profile_for_app(&self, app_id: &str) -> Option<&AppProfile> {
        self.profiles.iter().find(|p| p.app_id == app_id)
    }
}
