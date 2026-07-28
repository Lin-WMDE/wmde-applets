// SPDX-License-Identifier: GPL-3.0-only

use cosmic_config::{self, CosmicConfigEntry, cosmic_config_derive::CosmicConfigEntry};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum TemperatureUnit {
    Celsius,
    Fahrenheit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum WindUnit {
    MetersPerSecond,
    KilometersPerHour,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum PressureUnit {
    Hectopascal,
    MillimetersOfMercury,
    InchesOfMercury,
}

#[derive(Debug, Clone, CosmicConfigEntry, PartialEq, serde::Deserialize, serde::Serialize)]
#[version = 1]
pub struct WeatherAppletConfig {
    pub latitude: f64,
    pub longitude: f64,
    pub city_name: String,
    pub update_interval_minutes: u32,
    pub unit: TemperatureUnit,
    pub wind_unit: WindUnit,
    pub pressure_unit: PressureUnit,
}

impl Default for WeatherAppletConfig {
    fn default() -> Self {
        Self {
            latitude: 50.4501,
            longitude: 30.5234,
            city_name: String::from("Kyiv"),
            update_interval_minutes: 10,
            unit: TemperatureUnit::Celsius,
            wind_unit: WindUnit::MetersPerSecond,
            pressure_unit: PressureUnit::Hectopascal,
        }
    }
}
