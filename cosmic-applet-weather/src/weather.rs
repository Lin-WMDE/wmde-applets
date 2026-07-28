// SPDX-License-Identifier: GPL-3.0-only

//! Open-Meteo client, data model and unit formatting.
//!
//! Endpoints, field sets and number formats mirror the reference Nimbus
//! implementation (internal/weather) so the applet shows the same values
//! the same way.

use cosmic_applets_config::weather::{PressureUnit, TemperatureUnit, WindUnit};
use serde::Deserialize;
use std::sync::OnceLock;
use std::time::Duration;

const FORECAST_URL: &str = "https://api.open-meteo.com/v1/forecast";
const GEOCODING_URL: &str = "https://geocoding-api.open-meteo.com/v1/search";

/// Glyphs from the embedded Weather Icons font (private-use codepoints).
pub const GLYPH_DAY_SUNNY: char = '\u{f00d}';
pub const GLYPH_DAY_CLOUDY: char = '\u{f002}';
pub const GLYPH_CLOUD: char = '\u{f041}';
pub const GLYPH_FOG: char = '\u{f014}';
pub const GLYPH_RAIN: char = '\u{f019}';
pub const GLYPH_SHOWERS: char = '\u{f01a}';
pub const GLYPH_SNOW: char = '\u{f01b}';
pub const GLYPH_THUNDERSTORM: char = '\u{f01e}';

fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("failed to build HTTP client")
    })
}

#[derive(Debug, Clone, Deserialize)]
pub struct CurrentWeather {
    #[serde(rename = "temperature_2m")]
    pub temperature_c: f64,
    #[serde(rename = "apparent_temperature")]
    pub feels_like_c: f64,
    #[serde(rename = "relative_humidity_2m")]
    pub humidity: f64,
    #[serde(rename = "surface_pressure")]
    pub pressure_hpa: f64,
    #[serde(rename = "wind_speed_10m")]
    pub wind_kmh: f64,
    pub weather_code: i64,
}

#[derive(Debug, Deserialize)]
struct CurrentResponse {
    current: CurrentWeather,
}

#[derive(Debug, Deserialize)]
struct DailyResponse {
    daily: DailyBlock,
}

#[derive(Debug, Deserialize)]
struct DailyBlock {
    time: Vec<String>,
    temperature_2m_max: Vec<f64>,
    temperature_2m_min: Vec<f64>,
    weather_code: Vec<i64>,
    precipitation_sum: Vec<f64>,
    wind_speed_10m_max: Vec<f64>,
}

#[derive(Debug, Clone)]
pub struct DayForecast {
    /// ISO date as returned by the API, e.g. "2026-07-28".
    pub date: String,
    pub code: i64,
    pub temp_max_c: f64,
    pub temp_min_c: f64,
    pub precip_mm: f64,
    pub wind_kmh: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CityMatch {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    #[serde(default)]
    pub country: String,
    #[serde(default)]
    pub admin1: String,
}

#[derive(Debug, Deserialize)]
struct GeocodingResponse {
    #[serde(default)]
    results: Vec<CityMatch>,
}

async fn get_json<T: serde::de::DeserializeOwned>(
    url: &str,
    query: &[(&str, String)],
) -> Result<T, reqwest::Error> {
    client()
        .get(url)
        .query(query)
        .send()
        .await?
        .error_for_status()?
        .json::<T>()
        .await
}

pub async fn fetch_current(lat: f64, lon: f64) -> Option<CurrentWeather> {
    let query = [
        ("latitude", format!("{lat:.4}")),
        ("longitude", format!("{lon:.4}")),
        (
            "current",
            String::from(
                "temperature_2m,relative_humidity_2m,apparent_temperature,\
                 weather_code,wind_speed_10m,surface_pressure",
            ),
        ),
        ("timezone", String::from("auto")),
    ];
    match get_json::<CurrentResponse>(FORECAST_URL, &query).await {
        Ok(r) => Some(r.current),
        Err(err) => {
            tracing::error!(?err, "failed to fetch current weather");
            None
        }
    }
}

pub async fn fetch_daily(lat: f64, lon: f64) -> Option<Vec<DayForecast>> {
    let query = [
        ("latitude", format!("{lat:.4}")),
        ("longitude", format!("{lon:.4}")),
        (
            "daily",
            String::from(
                "temperature_2m_max,temperature_2m_min,weather_code,\
                 precipitation_sum,wind_speed_10m_max",
            ),
        ),
        ("timezone", String::from("auto")),
        ("forecast_days", String::from("7")),
    ];
    match get_json::<DailyResponse>(FORECAST_URL, &query).await {
        Ok(r) => {
            let d = r.daily;
            // Bound by the shortest of the parallel arrays: the API has
            // returned ragged arrays in the wild and that must not panic.
            let n = d
                .time
                .len()
                .min(d.temperature_2m_max.len())
                .min(d.temperature_2m_min.len())
                .min(d.weather_code.len())
                .min(d.precipitation_sum.len())
                .min(d.wind_speed_10m_max.len());
            Some(
                (0..n)
                    .map(|i| DayForecast {
                        date: d.time[i].clone(),
                        code: d.weather_code[i],
                        temp_max_c: d.temperature_2m_max[i],
                        temp_min_c: d.temperature_2m_min[i],
                        precip_mm: d.precipitation_sum[i],
                        wind_kmh: d.wind_speed_10m_max[i],
                    })
                    .collect(),
            )
        }
        Err(err) => {
            tracing::error!(?err, "failed to fetch daily forecast");
            None
        }
    }
}

pub async fn search_city(query: String, lang: String) -> Option<Vec<CityMatch>> {
    let query = [
        ("name", query),
        ("count", String::from("15")),
        ("language", lang),
        ("format", String::from("json")),
    ];
    match get_json::<GeocodingResponse>(GEOCODING_URL, &query).await {
        Ok(r) => Some(r.results),
        Err(err) => {
            tracing::error!(?err, "city search failed");
            None
        }
    }
}

pub fn glyph_for_code(code: i64) -> char {
    match code {
        0 => GLYPH_DAY_SUNNY,
        1..=2 => GLYPH_DAY_CLOUDY,
        45..=48 => GLYPH_FOG,
        51..=57 => GLYPH_RAIN,
        61..=65 | 80..=86 => GLYPH_SHOWERS,
        71..=77 => GLYPH_SNOW,
        c if c >= 95 => GLYPH_THUNDERSTORM,
        _ => GLYPH_CLOUD,
    }
}

pub fn temp_to_unit(celsius: f64, unit: TemperatureUnit) -> f64 {
    match unit {
        TemperatureUnit::Celsius => celsius,
        TemperatureUnit::Fahrenheit => celsius * 9.0 / 5.0 + 32.0,
    }
}

pub fn temp_unit_label(unit: TemperatureUnit) -> &'static str {
    match unit {
        TemperatureUnit::Celsius => "\u{b0}C",
        TemperatureUnit::Fahrenheit => "\u{b0}F",
    }
}

/// Panel-button temperature: rounded integer with an explicit plus sign
/// when above zero ("+24\u{b0}", "-3\u{b0}", "0\u{b0}").
pub fn format_temp_short(celsius: f64, unit: TemperatureUnit) -> String {
    let v = temp_to_unit(celsius, unit).round();
    if v > 0.0 {
        format!("+{v:.0}\u{b0}")
    } else if v < 0.0 {
        format!("{v:.0}\u{b0}")
    } else {
        String::from("0\u{b0}")
    }
}

/// Forecast-table temperature range, always signed: "+24/+17\u{b0}C".
pub fn format_temp_range(max_c: f64, min_c: f64, unit: TemperatureUnit) -> String {
    format!(
        "{:+.0}/{:+.0}{}",
        temp_to_unit(max_c, unit),
        temp_to_unit(min_c, unit),
        temp_unit_label(unit)
    )
}

pub fn wind_to_unit(kmh: f64, unit: WindUnit) -> f64 {
    match unit {
        WindUnit::MetersPerSecond => kmh / 3.6,
        WindUnit::KilometersPerHour => kmh,
    }
}

/// "3.4 m/s" with a localized unit label supplied by the caller.
pub fn format_wind(kmh: f64, unit: WindUnit, label: &str) -> String {
    format!("{:.1} {label}", wind_to_unit(kmh, unit))
}

/// "0.3 mm" with a localized unit label supplied by the caller.
pub fn format_precip(mm: f64, label: &str) -> String {
    format!("{mm:.1} {label}")
}

/// Pressure with a localized unit label supplied by the caller.
pub fn format_pressure(hpa: f64, unit: PressureUnit, label: &str) -> String {
    match unit {
        PressureUnit::Hectopascal => format!("{hpa:.0} {label}"),
        PressureUnit::MillimetersOfMercury => format!("{:.0} {label}", hpa * 0.750064),
        PressureUnit::InchesOfMercury => format!("{:.2} {label}", hpa * 0.02953),
    }
}
