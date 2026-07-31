// SPDX-License-Identifier: GPL-3.0-only

//! The panel applet: a strip of graphs, a tooltip, and nothing else.
//!
//! There is no popup and no window. A click does nothing, exactly as in the
//! MATE applet this follows; the numbers behind the graphs are in the
//! tooltip, and the settings live in the Settings page built from
//! `data/applet-settings/fun.wmde.AppletSysmon.ron`.
//!
//! Sampling happens inline in `update`: reading five `/proc` files takes
//! microseconds, and handing that to `spawn_blocking` would cost more in
//! wakeups than the work itself.

use crate::fl;
use crate::graph::{self, Colors, Kind, MAX_BANDS, Metrics, Ring, Strip};
use crate::probe::{Probe, Sample, Sources};
use cosmic::{
    Element, Task, app,
    cosmic_theme::Spacing,
    iced::{
        Length, Subscription,
        futures::{SinkExt, channel::mpsc},
        stream,
        widget::canvas,
    },
    surface, theme,
    widget::{autosize, container},
};
use cosmic_applets_config::sysmon::SysmonAppletConfig;
use cosmic_config::CosmicConfigEntry;
use std::sync::LazyLock;
use std::time::Duration;

/// Bounds the settings page also declares. A width below this draws nothing
/// readable; above it the applet would eat the panel.
const MIN_WIDTH: u32 = 10;
const MAX_WIDTH: u32 = 200;
const MIN_INTERVAL: u32 = 250;
const MAX_INTERVAL: u32 = 10_000;

static AUTOSIZE_MAIN_ID: LazyLock<cosmic::widget::Id> =
    LazyLock::new(|| cosmic::widget::Id::new("autosize-main"));

pub fn run() -> cosmic::iced::Result {
    cosmic::applet::run::<SysmonApplet>(())
}

struct SysmonApplet {
    core: cosmic::app::Core,
    config: SysmonAppletConfig,
    probe: Probe,
    /// One history per graph, indexed by `Kind::index`, alive whether the
    /// graph is shown or not - switching one back on costs no allocation.
    rings: [Ring; Kind::COUNT],
    /// Enabled graphs in panel order. Rebuilt only when the config changes.
    kinds: Vec<Kind>,
    colors: Colors,
    /// Cleared once per tick; that is the only thing that invalidates it.
    cache: canvas::Cache,
    last: Sample,
}

#[derive(Debug, Clone)]
enum Message {
    Tick,
    /// Boxed: fifteen color strings would otherwise pad every `Tick` that
    /// goes through the queue to the size of the whole config.
    ConfigChanged(Box<SysmonAppletConfig>),
    Surface(surface::Action),
}

impl SysmonApplet {
    fn sources(&self) -> Sources {
        Sources {
            cpu: self.config.cpu,
            memory: self.config.memory,
            network: self.config.network,
            swap: self.config.swap,
            load: self.config.load,
            disk: self.config.disk,
        }
    }

    fn width(&self) -> u32 {
        self.config.graph_width.clamp(MIN_WIDTH, MAX_WIDTH)
    }

    fn interval(&self) -> u32 {
        self.config.interval_ms.clamp(MIN_INTERVAL, MAX_INTERVAL)
    }

    /// Reads the config into the shapes the tick and the draw need, so that
    /// neither has to look at strings or re-decide what is enabled.
    fn apply_config(&mut self) {
        let sources = self.sources();
        self.kinds.clear();
        for kind in Kind::ALL {
            let on = match kind {
                Kind::Cpu => sources.cpu,
                Kind::Memory => sources.memory,
                Kind::Network => sources.network,
                Kind::Swap => sources.swap,
                Kind::Load => sources.load,
                Kind::Disk => sources.disk,
            };
            if on {
                self.kinds.push(kind);
            }
        }

        let width = self.width() as usize;
        for kind in Kind::ALL {
            self.rings[kind.index()].set_capacity(width);
        }

        self.colors = Colors::default();
        for band in graph::Band::ALL {
            self.colors.bands[band as usize] = graph::parse_color(self.config.color(band.key()));
        }
        self.colors.background = graph::parse_color(&self.config.color_background);
        self.colors.border = graph::parse_color(&self.config.color_border);

        self.cache.clear();
    }

    fn push(&mut self, sample: Sample) {
        let mut values = [0.0f32; MAX_BANDS];
        for kind in &self.kinds {
            let count = match kind {
                Kind::Cpu => {
                    values[..4].copy_from_slice(&sample.cpu);
                    4
                }
                Kind::Memory => {
                    let total = sample.mem_total.max(1) as f32;
                    for (slot, bytes) in values.iter_mut().zip(sample.mem) {
                        *slot = bytes as f32 / total;
                    }
                    3
                }
                Kind::Network => {
                    values[0] = sample.net[0] as f32;
                    values[1] = sample.net[1] as f32;
                    2
                }
                Kind::Swap => {
                    values[0] = sample.swap_used as f32 / sample.swap_total.max(1) as f32;
                    1
                }
                Kind::Load => {
                    values[0] = sample.load;
                    1
                }
                Kind::Disk => {
                    values[0] = sample.disk[0] as f32;
                    values[1] = sample.disk[1] as f32;
                    2
                }
            };
            self.rings[kind.index()].push(&values[..count]);
        }
        self.last = sample;
    }

    /// One line per enabled graph, the way MATE puts the numbers behind the
    /// picture into the tooltip.
    fn tooltip_text(&self) -> String {
        if self.kinds.is_empty() {
            return fl!("tip-idle");
        }
        let s = &self.last;
        let mut out = String::with_capacity(96);
        for kind in &self.kinds {
            if !out.is_empty() {
                out.push('\n');
            }
            let line = match kind {
                Kind::Cpu => {
                    let busy: f32 = s.cpu.iter().sum();
                    fl!("tip-cpu", value = format!("{:.0}%", busy * 100.0))
                }
                Kind::Memory => fl!(
                    "tip-memory",
                    used = fmt_size(s.mem[0]),
                    total = fmt_size(s.mem_total)
                ),
                Kind::Network => fl!(
                    "tip-network",
                    rx = fmt_rate(s.net[0]),
                    tx = fmt_rate(s.net[1])
                ),
                Kind::Swap => fl!(
                    "tip-swap",
                    used = fmt_size(s.swap_used),
                    total = fmt_size(s.swap_total)
                ),
                Kind::Load => fl!("tip-load", value = format!("{:.2}", s.load)),
                Kind::Disk => fl!(
                    "tip-disk",
                    read = fmt_rate(s.disk[0]),
                    write = fmt_rate(s.disk[1])
                ),
            };
            out.push_str(&line);
        }
        out
    }
}

const KIB: f64 = 1024.0;
const MIB: f64 = 1024.0 * KIB;
const GIB: f64 = 1024.0 * MIB;

/// Binary units - the ones the kernel counts memory in.
fn fmt_size(bytes: u64) -> String {
    let value = bytes as f64;
    if value >= GIB {
        fl!("size-gib", value = format!("{:.1}", value / GIB))
    } else if value >= MIB {
        fl!("size-mib", value = format!("{:.1}", value / MIB))
    } else if value >= KIB {
        fl!("size-kib", value = format!("{:.0}", value / KIB))
    } else {
        fl!("size-b", value = bytes.to_string())
    }
}

fn fmt_rate(bytes_per_second: f64) -> String {
    if bytes_per_second >= MIB {
        fl!("rate-mib", value = format!("{:.1}", bytes_per_second / MIB))
    } else if bytes_per_second >= KIB {
        fl!("rate-kib", value = format!("{:.0}", bytes_per_second / KIB))
    } else {
        fl!("rate-b", value = format!("{:.0}", bytes_per_second))
    }
}

impl cosmic::Application for SysmonApplet {
    type Message = Message;
    type Executor = cosmic::SingleThreadExecutor;
    type Flags = ();
    const APP_ID: &'static str = "fun.wmde.AppletSysmon";

    fn init(core: cosmic::app::Core, _flags: Self::Flags) -> (Self, app::Task<Self::Message>) {
        let config = cosmic_config::Config::new(Self::APP_ID, SysmonAppletConfig::VERSION)
            .ok()
            .and_then(|c| SysmonAppletConfig::get_entry(&c).ok())
            .unwrap_or_default();

        let width = config.graph_width.clamp(MIN_WIDTH, MAX_WIDTH) as usize;
        let rings = Kind::ALL.map(|kind| Ring::new(kind.bands().len(), width));

        let mut applet = Self {
            core,
            config,
            probe: Probe::new(),
            rings,
            kinds: Vec::with_capacity(Kind::COUNT),
            colors: Colors::default(),
            cache: canvas::Cache::new(),
            last: Sample::default(),
        };
        applet.apply_config();

        (applet, Task::none())
    }

    fn core(&self) -> &cosmic::app::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::app::Core {
        &mut self.core
    }

    fn update(&mut self, message: Self::Message) -> app::Task<Self::Message> {
        match message {
            Message::Tick => {
                let sample = self.probe.sample(self.sources());
                self.push(sample);
                self.cache.clear();
            }
            Message::ConfigChanged(config) => {
                if *config != self.config {
                    self.config = *config;
                    self.apply_config();
                }
            }
            Message::Surface(action) => {
                return cosmic::task::message(cosmic::Action::Cosmic(
                    cosmic::app::Action::Surface(action),
                ));
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let horizontal = self.core.applet.is_horizontal();
        let suggested = self.core.applet.suggested_size(true);
        let (pad_major, pad_minor) = self.core.applet.suggested_padding(true);
        // Same axis mapping the applet Context uses for its own buttons.
        let (horizontal_padding, vertical_padding) = if horizontal {
            (pad_major, pad_minor)
        } else {
            (pad_minor, pad_major)
        };
        let Spacing { space_xxxs, .. } = theme::active().cosmic().spacing;

        let content: Element<'_, Message> = if self.kinds.is_empty() {
            // Nothing to draw, but the applet still has to be findable in the
            // panel - otherwise it cannot be selected and removed.
            cosmic::widget::icon::from_name("fun.wmde.AppletSysmon-symbolic")
                .size(suggested.0)
                .into()
        } else {
            Strip::new(
                &self.kinds,
                &self.rings,
                &self.colors,
                &self.cache,
                Metrics {
                    horizontal,
                    length: self.width() as f32,
                    thickness: f32::from(if horizontal { suggested.1 } else { suggested.0 }),
                    gap: f32::from(space_xxxs),
                },
            )
            .into()
        };

        let framed = if horizontal {
            container(content)
                .center_y(Length::Fixed(f32::from(suggested.1 + 2 * vertical_padding)))
                .padding([0, horizontal_padding])
        } else {
            container(content)
                .center_x(Length::Fixed(f32::from(
                    suggested.0 + 2 * horizontal_padding,
                )))
                .padding([vertical_padding, 0])
        };

        autosize::autosize(
            Element::from(self.core.applet.applet_tooltip(
                Element::from(framed),
                self.tooltip_text(),
                false,
                Message::Surface,
                None,
            )),
            AUTOSIZE_MAIN_ID.clone(),
        )
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        fn tick_subscription(interval_ms: u32) -> Subscription<Message> {
            Subscription::run_with(interval_ms, |interval_ms: &u32| {
                let period = Duration::from_millis(u64::from(*interval_ms));
                stream::channel(1, move |mut output: mpsc::Sender<Message>| async move {
                    loop {
                        let _ = output.send(Message::Tick).await;
                        tokio::time::sleep(period).await;
                    }
                })
            })
        }

        let config = self.core.watch_config(Self::APP_ID).map(|update| {
            for err in update.errors {
                tracing::error!(?err, "Error watching config");
            }
            Message::ConfigChanged(Box::new(update.config))
        });

        // With every graph switched off there is nothing to sample, so the
        // timer is not created at all.
        if self.kinds.is_empty() {
            config
        } else {
            Subscription::batch([tick_subscription(self.interval()), config])
        }
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}
