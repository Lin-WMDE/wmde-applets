// SPDX-License-Identifier: GPL-3.0-only

mod app;
mod localize;
mod settings;
pub mod weather;

use localize::localize;

/// Register the embedded Weather Icons face with the shared font system so
/// the condition glyphs render as ordinary text.
fn load_weather_font() {
    let mut font_system = cosmic::iced::advanced::graphics::text::font_system()
        .write()
        .unwrap();
    font_system.load_font(std::borrow::Cow::Borrowed(app::WEATHER_FONT_BYTES));
}

pub fn run() -> cosmic::iced::Result {
    localize();
    load_weather_font();
    app::run()
}

pub fn run_settings() -> cosmic::iced::Result {
    localize();
    settings::run()
}
