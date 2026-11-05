use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AodMode {
    #[default]
    AlwaysOn,
    Scheduled {
        start_hour: u8,
        start_minute: u8,
        end_hour: u8,
        end_minute: u8,
    },
    TimeoutSeconds(u32),
}

impl AodMode {
    pub fn is_active_at(&self, hour: u8, minute: u8) -> bool {
        match *self {
            AodMode::AlwaysOn | AodMode::TimeoutSeconds(_) => true,
            AodMode::Scheduled {
                start_hour,
                start_minute,
                end_hour,
                end_minute,
            } => {
                let current = hour as u32 * 60 + minute as u32;
                let start = start_hour as u32 * 60 + start_minute as u32;
                let end = end_hour as u32 * 60 + end_minute as u32;
                if start <= end {
                    current >= start && current <= end
                } else {
                    // Spans midnight (e.g. 22:00 to 07:00)
                    current >= start || current <= end
                }
            }
        }
    }

    pub fn timeout_seconds(&self) -> Option<u32> {
        match *self {
            AodMode::TimeoutSeconds(secs) => Some(secs),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ClockStyle {
    #[default]
    AppleMinimal,
    Bauhaus,
    FlipClock,
    Binary,
    RetroGlyph,
    CustomScript(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum WidgetAnimation {
    None,
    Pulse,
    #[default]
    Breathe,
    Shimmer,
    GlyphMorph,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetAppearanceConfig {
    pub color_rgba: [f32; 4],
    pub glow_intensity: f32, // 0.0 to 1.0
    pub transparency: f32,   // 0.0 to 1.0 (opacity)
    pub animation: WidgetAnimation,
    pub nerd_custom_params: Option<String>,
}

impl Default for WidgetAppearanceConfig {
    fn default() -> Self {
        Self {
            color_rgba: [1.0, 1.0, 1.0, 1.0],
            glow_intensity: 0.35,
            transparency: 1.0,
            animation: WidgetAnimation::Breathe,
            nerd_custom_params: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TransitionStyle {
    Fade,
    RadialWipe,
    GlyphExpand,
    SlideUp,
    #[default]
    Morph,
    Dissolve,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TransitionEasing {
    #[default]
    SmoothStep,
    Linear,
    EaseInOut,
    Spring,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionConfig {
    pub off_to_preview: TransitionStyle,
    pub preview_to_lock: TransitionStyle,
    pub lock_to_home: TransitionStyle,
    pub home_to_lock: TransitionStyle,
    pub duration_ms: u32,
    pub easing: TransitionEasing,
}

impl Default for TransitionConfig {
    fn default() -> Self {
        Self {
            off_to_preview: TransitionStyle::Fade,
            preview_to_lock: TransitionStyle::Morph,
            lock_to_home: TransitionStyle::SlideUp,
            home_to_lock: TransitionStyle::Fade,
            duration_ms: 350,
            easing: TransitionEasing::SmoothStep,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BurnInConfig {
    pub enabled: bool,
    pub shift_interval_seconds: u32,
    pub max_shift_px: i32,
}

impl Default for BurnInConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            shift_interval_seconds: 180, // Every 3 minutes
            max_shift_px: 3,             // +/- 3 pixels
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AodConfig {
    pub enabled: bool,
    pub mode: AodMode,
    pub clock_style: ClockStyle,
    pub appearance: WidgetAppearanceConfig,
    pub transitions: TransitionConfig,
    pub refresh_rate_hz: u32,
    pub burn_in_protection: BurnInConfig,
    pub dim_percentage: f32,
    pub show_weather: bool,
    pub show_battery: bool,
    pub show_notifications: bool,
    pub privacy_blur_notifications: bool,
    pub custom_rhai_script: Option<String>,
    pub suppress_oem_aod: bool,
}

impl Default for AodConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: AodMode::AlwaysOn,
            clock_style: ClockStyle::AppleMinimal,
            appearance: WidgetAppearanceConfig::default(),
            transitions: TransitionConfig::default(),
            refresh_rate_hz: 1, // 1Hz LTPO OLED target
            burn_in_protection: BurnInConfig::default(),
            dim_percentage: 0.15, // 15% brightness
            show_weather: true,
            show_battery: true,
            show_notifications: true,
            privacy_blur_notifications: true,
            custom_rhai_script: None,
            suppress_oem_aod: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BiometricAnimation {
    #[default]
    ParticleRipple,
    CyberpunkPulse,
    SolarFlare,
    MinimalistSweep,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BiometricIndicatorStyle {
    Ripples,
    #[default]
    NeonRings,
    HaloGlow,
    FingerprintGlyph,
    MinimalDot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdfpsCalibration {
    pub x_percent: f32,
    pub y_percent: f32,
    pub radius_px: f32,
    pub haptic_strength: f32,
    pub show_indicator: bool,
    pub indicator_style: BiometricIndicatorStyle,
}

impl Default for UdfpsCalibration {
    fn default() -> Self {
        Self {
            x_percent: 0.50,
            y_percent: 0.82,
            radius_px: 90.0,
            haptic_strength: 0.8,
            show_indicator: true,
            indicator_style: BiometricIndicatorStyle::NeonRings,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceUnlockConfig {
    pub show_icon: bool,
    pub retry_button_enabled: bool,
    pub retry_x_percent: f32,
    pub retry_y_percent: f32,
    pub timeout_ms: u32,
}

impl Default for FaceUnlockConfig {
    fn default() -> Self {
        Self {
            show_icon: true,
            retry_button_enabled: true,
            retry_x_percent: 0.50,
            retry_y_percent: 0.18,
            timeout_ms: 3000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ActionButtonType {
    Flashlight,
    Camera,
    SoundToggle,
    Calculator,
    CustomIntent(String),
    #[default]
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionButtonConfig {
    pub action_type: ActionButtonType,
    pub x_percent: f32,
    pub y_percent: f32,
    pub size_px: f32,
    pub enabled: bool,
}

impl ActionButtonConfig {
    pub fn new(action_type: ActionButtonType, x_percent: f32, y_percent: f32) -> Self {
        Self {
            action_type,
            x_percent,
            y_percent,
            size_px: 52.0,
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickActionsConfig {
    pub left_action: ActionButtonConfig,
    pub right_action: ActionButtonConfig,
    pub custom_actions: Vec<ActionButtonConfig>,
}

impl Default for QuickActionsConfig {
    fn default() -> Self {
        Self {
            left_action: ActionButtonConfig::new(ActionButtonType::Flashlight, 0.15, 0.92),
            right_action: ActionButtonConfig::new(ActionButtonType::Camera, 0.85, 0.92),
            custom_actions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FallbackCredentialType {
    #[default]
    SystemKeyguardPin,
    SystemKeyguardPattern,
    SystemKeyguardPassword,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockScreenConfig {
    pub enabled: bool,
    pub sync_with_aod: bool,
    pub independent_appearance: Option<WidgetAppearanceConfig>,
    pub udfps_calibration: UdfpsCalibration,
    pub biometric_animation: BiometricAnimation,
    pub face_unlock: FaceUnlockConfig,
    pub quick_actions: QuickActionsConfig,
    pub fallback_credential_type: FallbackCredentialType,
    pub nerd_layout_script: Option<String>,
}

impl Default for LockScreenConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sync_with_aod: true,
            independent_appearance: None,
            udfps_calibration: UdfpsCalibration::default(),
            biometric_animation: BiometricAnimation::ParticleRipple,
            face_unlock: FaceUnlockConfig::default(),
            quick_actions: QuickActionsConfig::default(),
            fallback_credential_type: FallbackCredentialType::SystemKeyguardPin,
            nerd_layout_script: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum WallpaperMode {
    #[default]
    Image,
    Video,
    LiveCanvas,
    MultiLayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ScreenFillMode {
    #[default]
    AspectFill,
    FitLetterbox,
    CustomZoom {
        zoom: u32,
        offset_x: i32,
        offset_y: i32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthEffectConfig {
    pub enabled: bool,
    pub depth_threshold: f32,
    pub clock_z_layer: f32,
    pub parallax_k_bg: f32,
    pub parallax_k_fg: f32,
}

impl Default for DepthEffectConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            depth_threshold: 0.55,
            clock_z_layer: 0.50,
            parallax_k_bg: 12.0,
            parallax_k_fg: 24.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoWallpaperConfig {
    pub video_path: String,
    pub fill_mode: ScreenFillMode,
    pub parallax_swipe_factor: f32,
    pub gyro_parallax_factor: f32,
    pub loop_playback: bool,
}

impl Default for VideoWallpaperConfig {
    fn default() -> Self {
        Self {
            video_path: String::new(),
            fill_mode: ScreenFillMode::AspectFill,
            parallax_swipe_factor: 0.35,
            gyro_parallax_factor: 8.0,
            loop_playback: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ShaderPreset {
    InteractiveFluid,
    NeonRain,
    Constellation,
    #[default]
    HoloAurora,
    DynamicParticles,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveShaderConfig {
    pub shader_preset: ShaderPreset,
    pub particle_density: u32,
    pub gyro_reactivity: f32,
    pub reactive_transitions: bool,
}

impl Default for LiveShaderConfig {
    fn default() -> Self {
        Self {
            shader_preset: ShaderPreset::HoloAurora,
            particle_density: 120,
            gyro_reactivity: 1.0,
            reactive_transitions: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayerType {
    BaseImage,
    BaseVideo,
    DepthClockOcclusion,
    ParticleShader(ShaderPreset),
    CustomOverlay(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerStateBehavior {
    pub opacity: f32,
    pub blur_radius_px: f32,
    pub scale: f32,
    pub hidden: bool,
    pub speed_multiplier: f32,
}

impl Default for LayerStateBehavior {
    fn default() -> Self {
        Self {
            opacity: 1.0,
            blur_radius_px: 0.0,
            scale: 1.0,
            hidden: false,
            speed_multiplier: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerStateTransitions {
    pub on_preview: LayerStateBehavior,
    pub on_lock: LayerStateBehavior,
    pub on_home: LayerStateBehavior,
}

impl Default for LayerStateTransitions {
    fn default() -> Self {
        Self {
            on_preview: LayerStateBehavior {
                opacity: 0.15,
                blur_radius_px: 0.0,
                scale: 0.95,
                hidden: false,
                speed_multiplier: 0.1,
            },
            on_lock: LayerStateBehavior {
                opacity: 1.0,
                blur_radius_px: 0.0,
                scale: 1.0,
                hidden: false,
                speed_multiplier: 1.0,
            },
            on_home: LayerStateBehavior {
                opacity: 1.0,
                blur_radius_px: 24.0, // Gaussian blur on launcher for legibility
                scale: 1.05,
                hidden: false,
                speed_multiplier: 0.5,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WallpaperLayer {
    pub id: String,
    pub name: String,
    pub layer_type: LayerType,
    pub fill_mode: ScreenFillMode,
    pub gyro_reactivity_x: f32,
    pub gyro_reactivity_y: f32,
    pub state_transitions: LayerStateTransitions,
}

impl WallpaperLayer {
    pub fn new_base_image(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            layer_type: LayerType::BaseImage,
            fill_mode: ScreenFillMode::AspectFill,
            gyro_reactivity_x: 6.0,
            gyro_reactivity_y: 6.0,
            state_transitions: LayerStateTransitions::default(),
        }
    }

    pub fn new_shader_overlay(id: &str, name: &str, preset: ShaderPreset) -> Self {
        let mut transitions = LayerStateTransitions::default();
        transitions.on_preview.hidden = true; // Hide heavy particles on AOD
        Self {
            id: id.to_string(),
            name: name.to_string(),
            layer_type: LayerType::ParticleShader(preset),
            fill_mode: ScreenFillMode::AspectFill,
            gyro_reactivity_x: 12.0,
            gyro_reactivity_y: 12.0,
            state_transitions: transitions,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiLayerConfig {
    pub enabled: bool,
    pub layers: Vec<WallpaperLayer>,
}

impl Default for MultiLayerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            layers: vec![
                WallpaperLayer::new_base_image("layer_0_bg", "Base Image"),
                WallpaperLayer::new_shader_overlay(
                    "layer_1_particles",
                    "Interactive Particles",
                    ShaderPreset::HoloAurora,
                ),
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WallpaperConfig {
    pub wallpaper_mode: WallpaperMode,
    pub image_path: Option<String>,
    pub depth_effect: DepthEffectConfig,
    pub video_config: VideoWallpaperConfig,
    pub live_shader: LiveShaderConfig,
    pub multilayer_config: MultiLayerConfig,
}

impl Default for WallpaperConfig {
    fn default() -> Self {
        Self {
            wallpaper_mode: WallpaperMode::Image,
            image_path: None,
            depth_effect: DepthEffectConfig::default(),
            video_config: VideoWallpaperConfig::default(),
            live_shader: LiveShaderConfig::default(),
            multilayer_config: MultiLayerConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerConfig {
    pub oled_true_black: bool,
    pub device_idle_whitelisted: bool,
    pub app_standby_exempt: bool,
    pub min_wake_lock: bool,
    pub ltpo_refresh_rate_negotiation: bool,
}

impl Default for PowerConfig {
    fn default() -> Self {
        Self {
            oled_true_black: true,
            device_idle_whitelisted: true,
            app_standby_exempt: true,
            min_wake_lock: true,
            ltpo_refresh_rate_negotiation: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChronosConfig {
    pub aod: AodConfig,
    pub lockscreen: LockScreenConfig,
    pub wallpaper: WallpaperConfig,
    pub power: PowerConfig,
}

impl ChronosConfig {
    pub fn to_bincode(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    pub fn from_bincode(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let json = self
            .to_json()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let mut file = File::create(path)?;
        file.write_all(json.as_bytes())?;
        file.sync_all()?;
        Ok(())
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let mut file = File::open(path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        Self::from_json(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = ChronosConfig::default();
        assert!(config.aod.enabled);
        assert_eq!(config.aod.refresh_rate_hz, 1);
        assert!(config.power.oled_true_black);
        assert_eq!(
            config.lockscreen.biometric_animation,
            BiometricAnimation::ParticleRipple
        );
        assert_eq!(config.wallpaper.multilayer_config.layers.len(), 2);
    }

    #[test]
    fn test_aod_schedule_evaluation() {
        // Daytime schedule: 08:00 to 22:00
        let day_mode = AodMode::Scheduled {
            start_hour: 8,
            start_minute: 0,
            end_hour: 22,
            end_minute: 0,
        };
        assert!(day_mode.is_active_at(12, 30));
        assert!(!day_mode.is_active_at(7, 30));
        assert!(!day_mode.is_active_at(23, 15));

        // Night schedule spanning midnight: 22:00 to 06:00
        let night_mode = AodMode::Scheduled {
            start_hour: 22,
            start_minute: 0,
            end_hour: 6,
            end_minute: 0,
        };
        assert!(night_mode.is_active_at(23, 0));
        assert!(night_mode.is_active_at(2, 0));
        assert!(!night_mode.is_active_at(12, 0));
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let original = ChronosConfig::default();
        let bincode_data = original.to_bincode().expect("serialization failed");
        let deserialized =
            ChronosConfig::from_bincode(&bincode_data).expect("deserialization failed");
        assert_eq!(
            original.aod.refresh_rate_hz,
            deserialized.aod.refresh_rate_hz
        );
        assert_eq!(original.aod.dim_percentage, deserialized.aod.dim_percentage);
        assert_eq!(deserialized.wallpaper.multilayer_config.layers.len(), 2);

        let json_str = original.to_json().expect("json failed");
        let from_json = ChronosConfig::from_json(&json_str).expect("from json failed");
        assert_eq!(from_json.lockscreen.udfps_calibration.x_percent, 0.50);
        assert_eq!(from_json.aod.appearance.glow_intensity, 0.35);
    }
}
