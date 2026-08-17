//! BlinkCue for macOS: menu-bar blink cue + 20-20-20 reminder.

mod animation;
mod app;
mod autostart;
mod break_banner;
mod idle;
mod overlay;
mod scheduler;
mod settings;
mod tray;

fn main() {
    app::run();
}
