// Release builds on Windows open no console window.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod describe;
mod mapping_view;
mod matrix;
mod netmap;
mod network;
mod settings;
mod settings_view;
mod text;
mod topology;
mod vendor;
mod view;

use std::sync::{Mutex, OnceLock};

use iced::futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use scramble_ui::{font, omarchy};

/// Events from background threads, delivered through a subscription.
#[derive(Debug, Clone)]
pub enum External {
    OmarchyThemeChanged,
    Interfaces(Vec<avb_net::Interface>),
    Network(network::Report),
}

static EXTERNAL_EVENTS: Mutex<Option<UnboundedReceiver<External>>> = Mutex::new(None);
static EXTERNAL_SENDER: OnceLock<UnboundedSender<External>> = OnceLock::new();

/// Sends `event` to the app from any thread.
pub fn post(event: External) {
    if let Some(sender) = EXTERNAL_SENDER.get() {
        let _ = sender.unbounded_send(event);
    }
}

/// How often interfaces are looked at again, for links going up or down
/// and adapters plugged in.
const INTERFACE_POLL: std::time::Duration = std::time::Duration::from_secs(2);

fn main() -> iced::Result {
    if let Some(argument) = std::env::args().nth(1) {
        match argument.as_str() {
            "--version" | "-V" => println!("triib {}", env!("CARGO_PKG_VERSION")),
            _ => println!("usage: triib [--version]"),
        }
        return Ok(());
    }

    let (sender, receiver) = mpsc::unbounded();
    let _ = EXTERNAL_SENDER.set(sender);
    *EXTERNAL_EVENTS
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = Some(receiver);

    let omarchy_dir = omarchy::current_theme_dir();
    if let Some(dir) = omarchy_dir.clone() {
        omarchy::watch(dir, || post(External::OmarchyThemeChanged));
    }
    watch_interfaces();

    iced::application(
        move || app::Triib::boot(omarchy_dir.clone()),
        app::Triib::update,
        app::Triib::view,
    )
    .settings(iced::Settings {
        id: Some(app::APP_ID.to_owned()),
        fonts: font::files().collect(),
        default_font: font::TEXT,
        default_text_size: font::DEFAULT_SIZE.into(),
        antialiasing: true,
        ..iced::Settings::default()
    })
    .window(iced::window::Settings {
        size: iced::Size::new(1280.0, 800.0),
        min_size: Some(iced::Size::new(360.0, 480.0)),
        ..iced::window::Settings::default()
    })
    .title(app::Triib::title)
    .theme(app::Triib::theme)
    .subscription(app::Triib::subscription)
    .run()
}

/// Posts the interface list whenever it changes.
fn watch_interfaces() {
    let _ = std::thread::Builder::new()
        .name("interface-watch".into())
        .spawn(|| {
            let mut last = Vec::new();
            loop {
                let now = avb_net::interfaces();
                if now != last {
                    post(External::Interfaces(now.clone()));
                    last = now;
                }
                std::thread::sleep(INTERFACE_POLL);
            }
        });
}

/// Hands the receiver to the subscription once.
fn external_events() -> impl iced::futures::Stream<Item = External> {
    let receiver = EXTERNAL_EVENTS
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .take();
    iced::futures::StreamExt::flatten(iced::futures::stream::iter(receiver))
}
