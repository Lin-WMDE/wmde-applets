// Copyright 2023 System76 <info@system76.com>
// SPDX-License-Identifier: GPL-3.0-only

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> cosmic::iced::Result {
    tracing_subscriber::fmt().with_env_filter("warn").init();
    let _ = tracing_log::LogTracer::init();

    let Some(applet) = std::env::args().next() else {
        return Ok(());
    };

    let start = applet.rfind('/').map_or(0, |v| v + 1);
    let cmd = &applet.as_str()[start..];

    tracing::info!("Starting `{cmd}` with version {VERSION}");

    match cmd {
        "wmde-app-list" => cosmic_app_list::run(),
        "wmde-applet-a11y" => cosmic_applet_a11y::run(),
        "wmde-applet-audio" => cosmic_applet_audio::run(),
        "wmde-applet-battery" => cosmic_applet_battery::run(),
        "wmde-applet-bluetooth" => cosmic_applet_bluetooth::run(),
        "wmde-applet-minimize" => cosmic_applet_minimize::run(),
        "wmde-applet-network" => cosmic_applet_network::run(),
        "wmde-applet-notifications" => cosmic_applet_notifications::run(),
        "wmde-applet-power" => cosmic_applet_power::run(),
        "wmde-applet-status-area" => cosmic_applet_status_area::run(),
        "wmde-applet-tiling" => cosmic_applet_tiling::run(),
        "wmde-applet-time" => cosmic_applet_time::run(),
        "wmde-applet-workspaces" => cosmic_applet_workspaces::run(),
        "wmde-applet-input-sources" => cosmic_applet_input_sources::run(),
        "wmde-panel-button" => cosmic_panel_button::run(),
        _ => Ok(()),
    }
}
