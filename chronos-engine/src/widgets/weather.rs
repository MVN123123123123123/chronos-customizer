//! Weather and AQI Display Widget

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeatherCondition {
    Sunny,
    PartlyCloudy,
    Rainy,
    Snowy,
    Thunderstorm,
}

#[derive(Debug, Clone)]
pub struct WeatherData {
    pub temperature_c: i32,
    pub condition: WeatherCondition,
    pub aqi: u32,
    pub location: String,
}

impl Default for WeatherData {
    fn default() -> Self {
        Self {
            temperature_c: 22,
            condition: WeatherCondition::Sunny,
            aqi: 28, // Good AQI
            location: "Local".to_string(),
        }
    }
}

pub struct WeatherWidget {
    pub data: WeatherData,
}

impl WeatherWidget {
    pub fn new(data: WeatherData) -> Self {
        Self { data }
    }

    pub fn formatted_summary(&self) -> String {
        let cond_icon = match self.data.condition {
            WeatherCondition::Sunny => "☀️",
            WeatherCondition::PartlyCloudy => "⛅",
            WeatherCondition::Rainy => "🌧️",
            WeatherCondition::Snowy => "❄️",
            WeatherCondition::Thunderstorm => "⛈️",
        };
        format!(
            "{} {}°C • AQI {}",
            cond_icon, self.data.temperature_c, self.data.aqi
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weather_summary() {
        let w = WeatherWidget::new(WeatherData::default());
        let s = w.formatted_summary();
        assert!(s.contains("22°C"));
        assert!(s.contains("AQI 28"));
    }
}
