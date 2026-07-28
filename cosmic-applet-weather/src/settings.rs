// SPDX-License-Identifier: GPL-3.0-only

//! Standalone settings window (installed as wmde-weather-settings):
//! city search via the Open-Meteo geocoding API, units and update
//! interval. It writes the shared applet config; the running applet
//! picks changes up through watch_config.

use crate::fl;
use crate::localize::LANGUAGE_LOADER;
use crate::weather::{self, CityMatch};
use cosmic::{
    Element, Task, app,
    iced::{Alignment, Length, Limits},
    theme,
    widget::{Column, button, container, radio, scrollable, text, text_input},
};
use cosmic_applets_config::weather::{
    PressureUnit, TemperatureUnit, WeatherAppletConfig, WindUnit,
};
use cosmic_config::CosmicConfigEntry;
use i18n_embed::LanguageLoader;

const INTERVALS: [u32; 6] = [5, 10, 30, 60, 720, 1440];

pub fn run() -> cosmic::iced::Result {
    let settings = cosmic::app::Settings::default()
        .size_limits(Limits::NONE.width(560.0).height(700.0))
        .resizable(Some(0.0));
    cosmic::app::run::<SettingsApp>(settings, ())
}

struct SettingsApp {
    core: cosmic::app::Core,
    handler: Option<cosmic_config::Config>,
    config: WeatherAppletConfig,
    city_query: String,
    results: Vec<CityMatch>,
    searching: bool,
    searched: bool,
    search_failed: bool,
    interval_labels: Vec<String>,
}

#[derive(Debug, Clone)]
enum Message {
    CityQueryChanged(String),
    Search,
    SearchResults(Option<Vec<CityMatch>>),
    SelectCity(usize),
    UnitSelected(TemperatureUnit),
    WindUnitSelected(WindUnit),
    PressureUnitSelected(PressureUnit),
    IntervalSelected(usize),
}

fn interval_label(minutes: u32) -> String {
    match minutes {
        5 => fl!("interval-5"),
        10 => fl!("interval-10"),
        30 => fl!("interval-30"),
        60 => fl!("interval-60"),
        720 => fl!("interval-720"),
        _ => fl!("interval-1440"),
    }
}

fn city_label(city: &CityMatch) -> String {
    let mut label = city.name.clone();
    if !city.admin1.is_empty() {
        label.push_str(", ");
        label.push_str(&city.admin1);
    }
    if !city.country.is_empty() {
        label.push_str(", ");
        label.push_str(&city.country);
    }
    label
}

impl SettingsApp {
    fn save(&self) {
        if let Some(handler) = self.handler.as_ref() {
            if let Err(err) = self.config.write_entry(handler) {
                tracing::error!(?err, "failed to write weather applet config");
            }
        } else {
            tracing::error!("config handler unavailable; weather config not persisted");
        }
    }
}

impl cosmic::Application for SettingsApp {
    type Message = Message;
    type Executor = cosmic::executor::Default;
    type Flags = ();
    const APP_ID: &'static str = "fun.wmde.AppletWeather";

    fn init(core: cosmic::app::Core, _flags: Self::Flags) -> (Self, app::Task<Self::Message>) {
        let handler = cosmic_config::Config::new(Self::APP_ID, WeatherAppletConfig::VERSION).ok();
        let config = handler
            .as_ref()
            .and_then(|c| WeatherAppletConfig::get_entry(c).ok())
            .unwrap_or_default();

        (
            Self {
                core,
                handler,
                config,
                city_query: String::new(),
                results: Vec::new(),
                searching: false,
                searched: false,
                search_failed: false,
                interval_labels: INTERVALS.iter().map(|m| interval_label(*m)).collect(),
            },
            Task::none(),
        )
    }

    fn core(&self) -> &cosmic::app::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::app::Core {
        &mut self.core
    }

    fn update(&mut self, message: Self::Message) -> app::Task<Self::Message> {
        match message {
            Message::CityQueryChanged(query) => {
                self.city_query = query;
            }
            Message::Search => {
                let query = self.city_query.trim().to_owned();
                if query.is_empty() || self.searching {
                    return Task::none();
                }
                self.searching = true;
                self.searched = true;
                self.search_failed = false;
                let lang = LANGUAGE_LOADER.current_language().language.as_str().to_owned();
                return cosmic::task::future(async move {
                    Message::SearchResults(weather::search_city(query, lang).await)
                });
            }
            Message::SearchResults(results) => {
                self.searching = false;
                match results {
                    Some(results) => self.results = results,
                    None => {
                        self.results.clear();
                        self.search_failed = true;
                    }
                }
            }
            Message::SelectCity(index) => {
                if let Some(city) = self.results.get(index) {
                    self.config.latitude = city.latitude;
                    self.config.longitude = city.longitude;
                    self.config.city_name = city.name.clone();
                    self.city_query = city.name.clone();
                    self.results.clear();
                    self.searched = false;
                    self.save();
                }
            }
            Message::UnitSelected(unit) => {
                self.config.unit = unit;
                self.save();
            }
            Message::WindUnitSelected(unit) => {
                self.config.wind_unit = unit;
                self.save();
            }
            Message::PressureUnitSelected(unit) => {
                self.config.pressure_unit = unit;
                self.save();
            }
            Message::IntervalSelected(index) => {
                if let Some(minutes) = INTERVALS.get(index) {
                    self.config.update_interval_minutes = *minutes;
                    self.save();
                }
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let spacing = theme::active().cosmic().spacing;

        let search_row = cosmic::iced::widget::row![
            text_input(fl!("city-placeholder"), &self.city_query)
                .on_input(Message::CityQueryChanged)
                .on_submit(|_| Message::Search)
                .width(Length::Fill),
            button::standard(fl!("search")).on_press_maybe(
                (!self.searching && !self.city_query.trim().is_empty())
                    .then_some(Message::Search)
            ),
        ]
        .spacing(spacing.space_xs)
        .align_y(Alignment::Center);

        let mut location = Column::new()
            .push(text::heading(fl!("city")))
            .push(search_row)
            .push(text::body(format!(
                "{}: {} ({:.4}, {:.4})",
                fl!("selected-city"),
                self.config.city_name,
                self.config.latitude,
                self.config.longitude
            )))
            .spacing(spacing.space_xs);

        if self.searching {
            location = location.push(text::body(fl!("searching")));
        } else if self.search_failed {
            location = location.push(text::body(fl!("search-failed")));
        } else if !self.results.is_empty() {
            let mut list = Column::new().spacing(2);
            for (i, city) in self.results.iter().enumerate() {
                list = list.push(
                    button::text(format!(
                        "{} ({:.2}, {:.2})",
                        city_label(city),
                        city.latitude,
                        city.longitude
                    ))
                    .on_press(Message::SelectCity(i)),
                );
            }
            location = location.push(
                scrollable(list).height(Length::Fixed(180.0)).width(Length::Fill),
            );
        } else if self.searched {
            location = location.push(text::body(fl!("no-results")));
        }

        let units = Column::new()
            .push(text::heading(fl!("temperature")))
            .push(
                cosmic::iced::widget::row![
                    radio(
                        text::body("\u{b0}C"),
                        TemperatureUnit::Celsius,
                        Some(self.config.unit),
                        Message::UnitSelected,
                    ),
                    radio(
                        text::body("\u{b0}F"),
                        TemperatureUnit::Fahrenheit,
                        Some(self.config.unit),
                        Message::UnitSelected,
                    ),
                ]
                .spacing(spacing.space_m),
            )
            .push(text::heading(fl!("wind")))
            .push(
                cosmic::iced::widget::row![
                    radio(
                        text::body(fl!("unit-ms")),
                        WindUnit::MetersPerSecond,
                        Some(self.config.wind_unit),
                        Message::WindUnitSelected,
                    ),
                    radio(
                        text::body(fl!("unit-kmh")),
                        WindUnit::KilometersPerHour,
                        Some(self.config.wind_unit),
                        Message::WindUnitSelected,
                    ),
                ]
                .spacing(spacing.space_m),
            )
            .push(text::heading(fl!("pressure")))
            .push(
                cosmic::iced::widget::row![
                    radio(
                        text::body(fl!("unit-hpa")),
                        PressureUnit::Hectopascal,
                        Some(self.config.pressure_unit),
                        Message::PressureUnitSelected,
                    ),
                    radio(
                        text::body(fl!("unit-mmhg")),
                        PressureUnit::MillimetersOfMercury,
                        Some(self.config.pressure_unit),
                        Message::PressureUnitSelected,
                    ),
                    radio(
                        text::body(fl!("unit-inhg")),
                        PressureUnit::InchesOfMercury,
                        Some(self.config.pressure_unit),
                        Message::PressureUnitSelected,
                    ),
                ]
                .spacing(spacing.space_m),
            )
            .spacing(spacing.space_xs);

        let interval = Column::new()
            .push(text::heading(fl!("update-interval")))
            .push(cosmic::widget::dropdown(
                &self.interval_labels,
                INTERVALS
                    .iter()
                    .position(|m| *m == self.config.update_interval_minutes),
                Message::IntervalSelected,
            ))
            .spacing(spacing.space_xs);

        let content = Column::new()
            .push(text::title3(fl!("settings-title")))
            .push(location)
            .push(units)
            .push(interval)
            .spacing(spacing.space_m)
            .padding(spacing.space_l)
            .max_width(560);

        container(scrollable(content))
            .width(Length::Fill)
            .center_x(Length::Fill)
            .into()
    }
}
