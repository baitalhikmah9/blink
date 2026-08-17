//! Per-screen eye-break banner. Click the banner after a 3s grace to dismiss.

use std::time::Instant;

use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSApplication, NSBackingStoreType, NSColor, NSEvent, NSEventType, NSFont, NSScreen,
    NSTextAlignment, NSTextField, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask,
    NSStatusWindowLevel,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

use crate::scheduler::break_timer;

const BANNER_W: f64 = 420.0;
const BANNER_H: f64 = 96.0;
const TOP_MARGIN: f64 = 48.0;
const GRACE_MS: u128 = 3000;

pub struct BreakBanner {
    windows: Vec<Retained<NSWindow>>,
    started: Option<Instant>,
    duration_secs: u32,
    countdown: bool,
}

impl BreakBanner {
    pub fn new(_mtm: MainThreadMarker) -> Self {
        Self {
            windows: Vec::new(),
            started: None,
            duration_secs: break_timer::DEFAULT_DURATION_SECS,
            countdown: true,
        }
    }

    pub fn is_showing(&self) -> bool {
        self.started.is_some()
    }

    pub fn show(&mut self, mtm: MainThreadMarker, duration_secs: u32, countdown: bool) {
        self.hide();
        self.duration_secs = duration_secs;
        self.countdown = countdown;
        self.started = Some(Instant::now());

        let screens = NSScreen::screens(mtm);
        for screen in screens.iter() {
            let frame = screen.frame();
            let x = frame.origin.x + (frame.size.width - BANNER_W) / 2.0;
            let y = frame.origin.y + frame.size.height - TOP_MARGIN - BANNER_H;
            let rect = NSRect::new(NSPoint::new(x, y), NSSize::new(BANNER_W, BANNER_H));

            let window = unsafe {
                NSWindow::initWithContentRect_styleMask_backing_defer(
                    NSWindow::alloc(mtm),
                    rect,
                    NSWindowStyleMask::Borderless,
                    NSBackingStoreType::Buffered,
                    false,
                )
            };
            let bg = NSColor::colorWithSRGBRed_green_blue_alpha(0.12, 0.12, 0.14, 0.92);
            window.setOpaque(false);
            window.setBackgroundColor(Some(&bg));
            window.setLevel(NSStatusWindowLevel + 1);
            window.setCollectionBehavior(
                NSWindowCollectionBehavior::CanJoinAllSpaces | NSWindowCollectionBehavior::Stationary,
            );
            window.setHasShadow(true);
            window.setIgnoresMouseEvents(false);

            let title = label(
                mtm,
                break_timer::MESSAGE,
                NSRect::new(NSPoint::new(16.0, 48.0), NSSize::new(BANNER_W - 32.0, 28.0)),
                17.0,
                true,
            );
            let subtitle = label(
                mtm,
                break_timer::SECONDARY_MESSAGE,
                NSRect::new(NSPoint::new(16.0, 28.0), NSSize::new(BANNER_W - 32.0, 20.0)),
                12.0,
                false,
            );
            let countdown_field = label(
                mtm,
                &format!("{duration_secs}s"),
                NSRect::new(NSPoint::new(16.0, 8.0), NSSize::new(BANNER_W - 32.0, 18.0)),
                12.0,
                false,
            );
            countdown_field.setTag(1);

            if let Some(content) = window.contentView() {
                content.addSubview(&title);
                content.addSubview(&subtitle);
                content.addSubview(&countdown_field);
            }
            window.orderFrontRegardless();
            self.windows.push(window);
        }
    }

    /// Returns true when the break has ended (timeout or click after grace).
    pub fn tick(&mut self, mtm: MainThreadMarker) -> bool {
        let Some(start) = self.started else {
            return false;
        };
        let elapsed_ms = start.elapsed().as_millis();
        let elapsed_s = start.elapsed().as_secs_f64();
        let remaining = (self.duration_secs as f64 - elapsed_s).ceil().max(0.0) as u32;

        if self.countdown {
            for w in &self.windows {
                if let Some(content) = w.contentView() {
                    if let Some(field) = content.viewWithTag(1) {
                        let text = if remaining == 0 {
                            "Done".to_string()
                        } else {
                            format!("{remaining}s")
                        };
                        let ns = NSString::from_str(&text);
                        let _: () = unsafe { objc2::msg_send![&*field, setStringValue: &*ns] };
                    }
                }
            }
        }

        let clicked = elapsed_ms >= GRACE_MS && any_window_clicked(mtm, &self.windows);
        if clicked || elapsed_s >= self.duration_secs as f64 {
            self.hide();
            return true;
        }
        false
    }

    pub fn hide(&mut self) {
        for w in self.windows.drain(..) {
            w.orderOut(None);
            w.close();
        }
        self.started = None;
    }
}

fn any_window_clicked(mtm: MainThreadMarker, windows: &[Retained<NSWindow>]) -> bool {
    let app = NSApplication::sharedApplication(mtm);
    let Some(event) = app.currentEvent() else {
        return false;
    };
    let ty = event.r#type();
    if ty != NSEventType::LeftMouseDown && ty != NSEventType::LeftMouseUp {
        return false;
    }
    let loc = NSEvent::mouseLocation();
    windows.iter().any(|w| {
        let f = w.frame();
        loc.x >= f.origin.x
            && loc.x <= f.origin.x + f.size.width
            && loc.y >= f.origin.y
            && loc.y <= f.origin.y + f.size.height
    })
}

fn label(
    mtm: MainThreadMarker,
    text: &str,
    frame: NSRect,
    size: f64,
    bold: bool,
) -> Retained<NSTextField> {
    let field = NSTextField::initWithFrame(NSTextField::alloc(mtm), frame);
    field.setStringValue(&NSString::from_str(text));
    field.setBezeled(false);
    field.setDrawsBackground(false);
    field.setEditable(false);
    field.setSelectable(false);
    field.setAlignment(NSTextAlignment::Left);
    field.setTextColor(Some(&NSColor::whiteColor()));
    let font = if bold {
        NSFont::boldSystemFontOfSize(size)
    } else {
        NSFont::systemFontOfSize(size)
    };
    field.setFont(Some(&font));
    field
}
