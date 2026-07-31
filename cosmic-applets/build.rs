use std::fs;
use xdgen::{App, Context, FluentString};

fn main() {
    let ctx = Context::new("../i18n/", "desktop_entries").unwrap();

    [
        (
            "fun.wmde.AppList",
            "cosmic-app-list",
            "cosmic-app-list-comment",
            "cosmic-app-list-keywords",
        ),
        (
            "fun.wmde.AppletA11y",
            "cosmic-applet-a11y",
            "cosmic-applet-a11y-comment",
            "cosmic-applet-a11y-keywords",
        ),
        (
            "fun.wmde.AppletAudio",
            "cosmic-applet-audio",
            "cosmic-applet-audio-comment",
            "cosmic-applet-audio-keywords",
        ),
        (
            "fun.wmde.AppletBattery",
            "cosmic-applet-battery",
            "cosmic-applet-battery-comment",
            "cosmic-applet-battery-keywords",
        ),
        (
            "fun.wmde.AppletBluetooth",
            "cosmic-applet-bluetooth",
            "cosmic-applet-bluetooth-comment",
            "cosmic-applet-bluetooth-keywords",
        ),
        (
            "fun.wmde.AppletInputSources",
            "cosmic-applet-input-sources",
            "cosmic-applet-input-sources-comment",
            "cosmic-applet-input-sources-keywords",
        ),
        (
            "fun.wmde.AppletMinimize",
            "cosmic-applet-minimize",
            "cosmic-applet-minimize-comment",
            "cosmic-applet-minimize-keywords",
        ),
        (
            "fun.wmde.AppletNetwork",
            "cosmic-applet-network",
            "cosmic-applet-network-comment",
            "cosmic-applet-network-keywords",
        ),
        (
            "fun.wmde.AppletNotifications",
            "cosmic-applet-notifications",
            "cosmic-applet-notifications-comment",
            "cosmic-applet-notifications-keywords",
        ),
        (
            "fun.wmde.AppletPower",
            "cosmic-applet-power",
            "cosmic-applet-power-comment",
            "cosmic-applet-power-keywords",
        ),
        (
            "fun.wmde.AppletStatusArea",
            "cosmic-applet-status-area",
            "cosmic-applet-status-area-comment",
            "cosmic-applet-status-area-keywords",
        ),
        (
            "fun.wmde.AppletSysmon",
            "cosmic-applet-sysmon",
            "cosmic-applet-sysmon-comment",
            "cosmic-applet-sysmon-keywords",
        ),
        (
            "fun.wmde.AppletTiling",
            "cosmic-applet-tiling",
            "cosmic-applet-tiling-comment",
            "cosmic-applet-tiling-keywords",
        ),
        (
            "fun.wmde.AppletTime",
            "cosmic-applet-time",
            "cosmic-applet-time-comment",
            "cosmic-applet-time-keywords",
        ),
        (
            "fun.wmde.AppletWeather",
            "cosmic-applet-weather",
            "cosmic-applet-weather-comment",
            "cosmic-applet-weather-keywords",
        ),
        (
            "fun.wmde.AppletWorkspaces",
            "cosmic-applet-workspaces",
            "cosmic-applet-workspaces-comment",
            "cosmic-applet-workspaces-keywords",
        ),
    ]
    .into_iter()
    .map(|(id, name, comment, keywords)| {
        let template_path = ["../", name, "/data/", id, ".desktop"].concat();

        let app = App::new(FluentString(name))
            .comment(FluentString(comment))
            .keywords(FluentString(keywords));

        (id, app.expand_desktop(&template_path, &ctx).unwrap())
    })
    .for_each(|(id, contents)| {
        let parent = "../target/xdgen/";
        fs::create_dir_all(parent).unwrap();
        fs::write([parent, id, ".desktop"].concat().as_str(), contents).unwrap();
    });
}
