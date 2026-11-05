//! Precision Clock Renderers with Event-Driven Updates

use chronos_common::config::{ClockStyle, WidgetAnimation, WidgetAppearanceConfig};

#[derive(Debug, Clone)]
pub struct ClockRenderData {
    pub hour_str: String,
    pub minute_str: String,
    pub second_str: String,
    pub date_str: String,
    pub is_dirty: bool,
    pub color_rgba: [f32; 4],
    pub glow_intensity: f32,
    pub opacity: f32,
    pub animation_scale: f32,
}

pub struct ClockWidget {
    pub style: ClockStyle,
    pub appearance: WidgetAppearanceConfig,
    pub last_rendered_second: u32,
    pub last_rendered_minute: u32,
}

impl ClockWidget {
    pub fn new(style: ClockStyle, appearance: WidgetAppearanceConfig) -> Self {
        Self {
            style,
            appearance,
            last_rendered_second: 999,
            last_rendered_minute: 999,
        }
    }

    /// Evaluates whether the clock needs a redraw.
    /// In AOD mode (refresh_rate 1Hz or static minutely), skips redraw if time hasn't changed.
    pub fn update_time(
        &mut self,
        hour: u32,
        minute: u32,
        second: u32,
        date_str: &str,
        high_precision: bool,
    ) -> Option<ClockRenderData> {
        let is_dirty = if high_precision {
            second != self.last_rendered_second
        } else {
            minute != self.last_rendered_minute
        };

        if !is_dirty {
            return None;
        }

        self.last_rendered_second = second;
        self.last_rendered_minute = minute;

        let (h_str, m_str, s_str) = match self.style {
            ClockStyle::AppleMinimal => (
                format!("{:02}", hour),
                format!("{:02}", minute),
                format!("{:02}", second),
            ),
            ClockStyle::Bauhaus => (
                format!("{:02}\n{:02}", hour, minute),
                "".to_string(),
                format!("{:02}", second),
            ),
            ClockStyle::Binary => (
                format!("{:06b}", hour),
                format!("{:06b}", minute),
                format!("{:06b}", second),
            ),
            ClockStyle::FlipClock => (
                format!("[{:02}]", hour),
                format!("[{:02}]", minute),
                format!("[{:02}]", second),
            ),
            ClockStyle::RetroGlyph => (
                format!("// {:02} : {:02} //", hour, minute),
                "".to_string(),
                format!("{:02}", second),
            ),
            ClockStyle::CustomScript(ref script) => {
                log::trace!("Evaluating custom clock script: {}", script);
                (
                    format!("{:02}", hour),
                    format!("{:02}", minute),
                    format!("{:02}", second),
                )
            }
        };

        // Compute dynamic animation scale based on appearance configuration
        let anim_scale = match self.appearance.animation {
            WidgetAnimation::None => 1.0,
            WidgetAnimation::Pulse => {
                // Subtle pulse on second tick
                1.0 + (second % 2) as f32 * 0.02
            }
            WidgetAnimation::Breathe => {
                // 4-second breathing cycle
                let phase = (second % 4) as f32 / 4.0 * std::f32::consts::PI * 2.0;
                1.0 + phase.sin() * 0.03
            }
            WidgetAnimation::Shimmer | WidgetAnimation::GlyphMorph => 1.0,
        };

        Some(ClockRenderData {
            hour_str: h_str,
            minute_str: m_str,
            second_str: s_str,
            date_str: date_str.to_string(),
            is_dirty: true,
            color_rgba: self.appearance.color_rgba,
            glow_intensity: self.appearance.glow_intensity,
            opacity: self.appearance.transparency,
            animation_scale: anim_scale,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clock_event_driven_tick() {
        let mut widget =
            ClockWidget::new(ClockStyle::AppleMinimal, WidgetAppearanceConfig::default());

        // First update must produce render data
        let res1 = widget.update_time(10, 30, 0, "Mon, Nov 5", false);
        assert!(res1.is_some());
        let data = res1.unwrap();
        assert_eq!(data.hour_str, "10");
        assert_eq!(data.minute_str, "30");
        assert_eq!(data.color_rgba, [1.0, 1.0, 1.0, 1.0]);

        // Same minute on low-precision AOD should return None (sleeping pipeline)
        let res2 = widget.update_time(10, 30, 15, "Mon, Nov 5", false);
        assert!(res2.is_none());

        // Minute change should trigger redraw
        let res3 = widget.update_time(10, 31, 0, "Mon, Nov 5", false);
        assert!(res3.is_some());
    }
}
