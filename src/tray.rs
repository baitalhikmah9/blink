//! Menu-bar status item.

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Sel};
use objc2::{sel, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSMenu, NSMenuItem, NSStatusBar, NSStatusItem, NSVariableStatusItemLength,
};
use objc2_foundation::NSString;

pub struct Tray {
    pub item: Retained<NSStatusItem>,
    pub pause_item: Retained<NSMenuItem>,
    pub blink_item: Retained<NSMenuItem>,
    pub break_item: Retained<NSMenuItem>,
}

impl Tray {
    pub fn new(mtm: MainThreadMarker, target: &AnyObject) -> Self {
        let bar = NSStatusBar::systemStatusBar();
        let item = bar.statusItemWithLength(NSVariableStatusItemLength);
        if let Some(button) = item.button(mtm) {
            button.setTitle(&NSString::from_str("◉"));
            button.setToolTip(Some(&NSString::from_str("Blink!")));
        }

        let menu = NSMenu::new(mtm);
        menu.setAutoenablesItems(false);

        let show_cue = item_with(mtm, "Show cue now", sel!(showCue:), target);
        let take_break = item_with(mtm, "Take eye break now", sel!(takeBreak:), target);
        menu.addItem(&show_cue);
        menu.addItem(&take_break);
        menu.addItem(&NSMenuItem::separatorItem(mtm));

        let pause = item_with(mtm, "Pause 15 min", sel!(pause15:), target);
        let resume = item_with(mtm, "Resume", sel!(resume:), target);
        menu.addItem(&pause);
        menu.addItem(&resume);
        menu.addItem(&NSMenuItem::separatorItem(mtm));

        let blink = item_with(mtm, "Blink cues: On", sel!(toggleBlink:), target);
        let brk = item_with(mtm, "Eye breaks: On", sel!(toggleBreak:), target);
        let login = item_with(mtm, "Start at login", sel!(toggleLogin:), target);
        menu.addItem(&blink);
        menu.addItem(&brk);
        menu.addItem(&login);
        menu.addItem(&NSMenuItem::separatorItem(mtm));
        menu.addItem(&item_with(mtm, "Quit Blink!", sel!(quit:), target));

        item.setMenu(Some(&menu));

        Self {
            item,
            pause_item: pause,
            blink_item: blink,
            break_item: brk,
        }
    }

    pub fn set_paused(&self, mtm: MainThreadMarker, paused: bool) {
        self.pause_item.setTitle(&NSString::from_str(if paused {
            "Paused"
        } else {
            "Pause 15 min"
        }));
        if let Some(button) = self.item.button(mtm) {
            button.setTitle(&NSString::from_str(if paused { "◌" } else { "◉" }));
        }
    }

    pub fn set_blink_enabled(&self, on: bool) {
        self.blink_item.setTitle(&NSString::from_str(if on {
            "Blink cues: On"
        } else {
            "Blink cues: Off"
        }));
    }

    pub fn set_break_enabled(&self, on: bool) {
        self.break_item.setTitle(&NSString::from_str(if on {
            "Eye breaks: On"
        } else {
            "Eye breaks: Off"
        }));
    }
}

fn item_with(
    mtm: MainThreadMarker,
    title: &str,
    action: Sel,
    target: &AnyObject,
) -> Retained<NSMenuItem> {
    let item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            &NSString::from_str(title),
            Some(action),
            &NSString::from_str(""),
        )
    };
    unsafe { item.setTarget(Some(target)) };
    item
}
