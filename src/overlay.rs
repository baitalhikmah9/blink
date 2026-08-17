//! Click-through edge-glow windows on every screen.

use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSBackingStoreType, NSColor, NSMainMenuWindowLevel, NSScreen, NSWindow,
    NSWindowCollectionBehavior, NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize};

enum Edge {
    Top,
    Bottom,
    Left,
    Right,
}

struct EdgeWindow {
    window: Retained<NSWindow>,
}

pub struct OverlaySet {
    edges: Vec<EdgeWindow>,
    color: (u8, u8, u8),
    intensity: f64,
    thickness: f64,
    visible: bool,
}

impl OverlaySet {
    pub fn new(
        mtm: MainThreadMarker,
        color: (u8, u8, u8),
        intensity: u8,
        thickness_px: i32,
    ) -> Self {
        let mut set = Self {
            edges: Vec::new(),
            color,
            intensity: intensity.clamp(0, 255) as f64 / 255.0,
            thickness: thickness_px.max(8) as f64,
            visible: false,
        };
        set.rebuild(mtm);
        set
    }

    pub fn rebuild(&mut self, mtm: MainThreadMarker) {
        self.visible = false;
        for ew in self.edges.drain(..) {
            ew.window.orderOut(None);
            ew.window.close();
        }
        let screens = NSScreen::screens(mtm);
        for screen in screens.iter() {
            let frame = screen.frame();
            for edge in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
                if let Some(win) = make_edge_window(mtm, frame, edge, self.color, self.thickness) {
                    self.edges.push(EdgeWindow { window: win });
                }
            }
        }
        self.set_opacity(0.0);
    }

    pub fn set_opacity(&mut self, opacity: f32) {
        let alpha = (opacity as f64).clamp(0.0, 1.0) * self.intensity;
        let want_visible = alpha > 0.001;
        for ew in &self.edges {
            ew.window.setAlphaValue(alpha);
            if want_visible && !self.visible {
                ew.window.orderFrontRegardless();
            } else if !want_visible && self.visible {
                ew.window.orderOut(None);
            }
        }
        self.visible = want_visible;
    }

    pub fn hide(&mut self) {
        self.set_opacity(0.0);
    }
}

fn make_edge_window(
    mtm: MainThreadMarker,
    screen: NSRect,
    edge: Edge,
    color: (u8, u8, u8),
    thickness: f64,
) -> Option<Retained<NSWindow>> {
    let (x, y, w, h) = match edge {
        Edge::Top => (
            screen.origin.x,
            screen.origin.y + screen.size.height - thickness,
            screen.size.width,
            thickness,
        ),
        Edge::Bottom => (
            screen.origin.x,
            screen.origin.y,
            screen.size.width,
            thickness,
        ),
        Edge::Left => (
            screen.origin.x,
            screen.origin.y,
            thickness,
            screen.size.height,
        ),
        Edge::Right => (
            screen.origin.x + screen.size.width - thickness,
            screen.origin.y,
            thickness,
            screen.size.height,
        ),
    };

    let rect = NSRect::new(NSPoint::new(x, y), NSSize::new(w, h));
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(mtm),
            rect,
            NSWindowStyleMask::Borderless,
            NSBackingStoreType::Buffered,
            false,
        )
    };

    let (r, g, b) = color;
    let ns_color = NSColor::colorWithSRGBRed_green_blue_alpha(
        r as f64 / 255.0,
        g as f64 / 255.0,
        b as f64 / 255.0,
        1.0,
    );

    window.setOpaque(false);
    window.setBackgroundColor(Some(&ns_color));
    window.setIgnoresMouseEvents(true);
    window.setLevel(NSMainMenuWindowLevel + 2);
    window.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::Stationary
            | NSWindowCollectionBehavior::IgnoresCycle
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::FullScreenDisallowsTiling,
    );
    window.setHasShadow(false);
    window.setHidesOnDeactivate(false);
    window.setAlphaValue(0.0);
    window.orderOut(None);
    Some(window)
}
