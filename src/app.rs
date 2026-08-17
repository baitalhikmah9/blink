//! Owns schedulers, overlay, banner, tray, and NSTimers.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSApplication, NSApplicationDidChangeScreenParametersNotification};
use objc2_foundation::{NSNotificationCenter, NSObject, NSObjectProtocol, NSTimer};

use crate::animation::{CueAnimation, DoubleEdgePulse};
use crate::autostart;
use crate::break_banner::BreakBanner;
use crate::idle;
use crate::overlay::OverlaySet;
use crate::scheduler::{
    break_timer, IntervalScheduler, SchedulerConfig, MAX_INTERVAL_SECS, MIN_INTERVAL_SECS,
};
use crate::settings::{self, Settings};
use crate::tray::Tray;

pub struct AppState {
    settings: Settings,
    blink: IntervalScheduler,
    eye_break: IntervalScheduler,
    overlay: OverlaySet,
    banner: BreakBanner,
    tray: Option<Tray>,
    cue: Option<ActiveCue>,
    blink_timer: Option<Retained<NSTimer>>,
    break_timer_obj: Option<Retained<NSTimer>>,
    frame_timer: Option<Retained<NSTimer>>,
    banner_timer: Option<Retained<NSTimer>>,
    target: Option<Retained<AppTarget>>,
}

struct ActiveCue {
    animation: DoubleEdgePulse,
    start: Instant,
}

impl AppState {
    fn new(mtm: MainThreadMarker) -> Self {
        let mut settings = settings::load();
        match autostart::is_enabled() {
            Ok(actual_login) if settings.behaviour.start_at_login != actual_login => {
                settings.behaviour.start_at_login = actual_login;
                if let Err(e) = settings::save(&settings) {
                    eprintln!("Blink! settings save failed: {e}");
                }
            }
            Ok(_) => {}
            Err(e) => eprintln!("Blink! start-at-login probe failed: {e}"),
        }

        let blink = IntervalScheduler::new(
            SchedulerConfig {
                enabled: settings.blink.enabled,
                base_interval: Duration::from_secs_f32(settings.blink.interval_secs),
                jitter: settings.blink.jitter,
            },
            Duration::from_secs(MIN_INTERVAL_SECS),
            Duration::from_secs(MAX_INTERVAL_SECS),
        );
        let eye_break = IntervalScheduler::new(
            SchedulerConfig {
                enabled: settings.eye_break.enabled,
                base_interval: Duration::from_secs_f32(settings.eye_break.interval_secs),
                jitter: crate::scheduler::Jitter::Off,
            },
            Duration::from_secs(break_timer::MIN_INTERVAL_SECS),
            Duration::from_secs(break_timer::MAX_INTERVAL_SECS),
        );
        let overlay = OverlaySet::new(
            mtm,
            settings.appearance.color,
            settings.appearance.intensity,
            settings.appearance.thickness_px,
        );
        let banner = BreakBanner::new(mtm);
        Self {
            settings,
            blink,
            eye_break,
            overlay,
            banner,
            tray: None,
            cue: None,
            blink_timer: None,
            break_timer_obj: None,
            frame_timer: None,
            banner_timer: None,
            target: None,
        }
    }

    fn persist(&self) {
        if let Err(e) = settings::save(&self.settings) {
            eprintln!("Blink! settings save failed: {e}");
        }
    }

    fn sync_pause_ui(&self, mtm: MainThreadMarker) {
        if let Some(tray) = &self.tray {
            tray.set_paused(mtm, self.blink.is_paused());
        }
    }

    fn sync_login_ui(&self) {
        if let Some(tray) = &self.tray {
            tray.set_login_enabled(self.settings.behaviour.start_at_login);
        }
    }

    fn idle_blocked(&self) -> bool {
        if !self.settings.behaviour.pause_when_idle {
            return false;
        }
        idle::seconds_since_input() >= self.settings.behaviour.idle_threshold_secs as f64
    }

    fn schedule_frame_timer(&mut self) {
        let Some(target) = self.target.as_ref() else {
            return;
        };
        let timer = unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                1.0 / 30.0,
                target,
                sel!(frameTick:),
                None,
                true,
            )
        };
        self.frame_timer = Some(timer);
    }

    fn schedule_banner_timer(&mut self) {
        let Some(target) = self.target.as_ref() else {
            return;
        };
        let timer = unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                0.2,
                target,
                sel!(bannerTick:),
                None,
                true,
            )
        };
        self.banner_timer = Some(timer);
    }

    fn show_cue(&mut self) {
        if self.cue.is_some() {
            return;
        }
        if self.target.is_none() {
            return;
        }
        if self.idle_blocked() {
            self.blink.reset_without_firing();
            return;
        }
        let anim = DoubleEdgePulse::from_total_duration(self.settings.appearance.duration_ms, 200);
        self.cue = Some(ActiveCue {
            animation: anim,
            start: Instant::now(),
        });
        self.schedule_frame_timer();
    }

    fn show_break(&mut self, mtm: MainThreadMarker) {
        if self.banner.is_showing() {
            return;
        }
        let Some(target) = self.target.as_ref() else {
            return;
        };
        if self.idle_blocked() {
            self.eye_break.reset_without_firing();
            return;
        }
        self.banner.show(
            mtm,
            self.settings.eye_break.duration_secs,
            self.settings.eye_break.countdown_enabled,
            target,
        );
        self.schedule_banner_timer();
        self.eye_break.reset_without_firing();
    }

    fn tick_cue(&mut self) {
        let Some(cue) = &self.cue else {
            self.overlay.hide();
            return;
        };
        let elapsed = cue.start.elapsed().as_millis() as u32;
        if elapsed >= cue.animation.total_duration_ms() {
            self.overlay.hide();
            self.cue = None;
            if let Some(timer) = self.frame_timer.take() {
                timer.invalidate();
            }
            return;
        }
        self.overlay.set_opacity(cue.animation.opacity_at(elapsed));
    }

    fn banner_tick(&mut self) {
        if self.banner.tick() {
            if let Some(timer) = self.banner_timer.take() {
                timer.invalidate();
            }
        }
    }

    fn banner_clicked(&mut self, _event: Option<&AnyObject>) {
        if self.banner.click() {
            if let Some(timer) = self.banner_timer.take() {
                timer.invalidate();
            }
        }
    }
}

pub struct AppIvars {
    state: RefCell<AppState>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "BlinkAppTarget"]
    #[ivars = AppIvars]
    pub struct AppTarget;

    unsafe impl NSObjectProtocol for AppTarget {}

    impl AppTarget {
        #[unsafe(method(showCue:))]
        fn show_cue_action(&self, _sender: Option<&AnyObject>) {
            self.ivars().state.borrow_mut().show_cue();
        }

        #[unsafe(method(takeBreak:))]
        fn take_break_action(&self, _sender: Option<&AnyObject>) {
            let mtm = MainThreadMarker::from(self);
            self.ivars().state.borrow_mut().show_break(mtm);
        }

        #[unsafe(method(pause15:))]
        fn pause15(&self, _sender: Option<&AnyObject>) {
            let mtm = MainThreadMarker::from(self);
            let mut st = self.ivars().state.borrow_mut();
            let was_paused = st.blink.is_paused();
            st.blink.pause_for(Duration::from_secs(15 * 60));
            let now_paused = st.blink.is_paused();
            if now_paused != was_paused {
                st.sync_pause_ui(mtm);
            }
        }

        #[unsafe(method(resume:))]
        fn resume(&self, _sender: Option<&AnyObject>) {
            let mtm = MainThreadMarker::from(self);
            let mut st = self.ivars().state.borrow_mut();
            let was_paused = st.blink.is_paused();
            st.blink.resume();
            let now_paused = st.blink.is_paused();
            if now_paused != was_paused {
                st.sync_pause_ui(mtm);
            }
        }

        #[unsafe(method(toggleBlink:))]
        fn toggle_blink(&self, _sender: Option<&AnyObject>) {
            let mtm = MainThreadMarker::from(self);
            let mut st = self.ivars().state.borrow_mut();
            let on = !st.settings.blink.enabled;
            st.settings.blink.enabled = on;
            st.blink.set_enabled(on);
            st.persist();
            if let Some(tray) = &st.tray {
                tray.set_blink_enabled(on);
            }
            st.sync_pause_ui(mtm);
        }

        #[unsafe(method(toggleBreak:))]
        fn toggle_break(&self, _sender: Option<&AnyObject>) {
            let mut st = self.ivars().state.borrow_mut();
            let on = !st.settings.eye_break.enabled;
            st.settings.eye_break.enabled = on;
            st.eye_break.set_enabled(on);
            st.persist();
            if let Some(tray) = &st.tray {
                tray.set_break_enabled(on);
            }
        }

        #[unsafe(method(toggleLogin:))]
        fn toggle_login(&self, _sender: Option<&AnyObject>) {
            let mut st = self.ivars().state.borrow_mut();
            let on = !st.settings.behaviour.start_at_login;
            match autostart::set_enabled(on) {
                Ok(()) => {
                    st.settings.behaviour.start_at_login = on;
                    st.persist();
                    st.sync_login_ui();
                }
                Err(e) => {
                    eprintln!("Blink! start-at-login failed: {e}");
                }
            }
        }

        #[unsafe(method(quit:))]
        fn quit(&self, _sender: Option<&AnyObject>) {
            let mtm = MainThreadMarker::from(self);
            NSApplication::sharedApplication(mtm).terminate(None);
        }

        #[unsafe(method(blinkDue:))]
        fn blink_due(&self, _timer: Option<&AnyObject>) {
            let mtm = MainThreadMarker::from(self);
            let mut st = self.ivars().state.borrow_mut();
            if st.blink.poll_due() {
                st.show_cue();
            }
            st.sync_pause_ui(mtm);
        }

        #[unsafe(method(breakDue:))]
        fn break_due(&self, _timer: Option<&AnyObject>) {
            let mtm = MainThreadMarker::from(self);
            let mut st = self.ivars().state.borrow_mut();
            if st.eye_break.poll_due() {
                st.show_break(mtm);
            }
        }

        #[unsafe(method(frameTick:))]
        fn frame_tick(&self, _timer: Option<&AnyObject>) {
            self.ivars().state.borrow_mut().tick_cue();
        }

        #[unsafe(method(bannerTick:))]
        fn banner_tick(&self, _timer: Option<&AnyObject>) {
            self.ivars().state.borrow_mut().banner_tick();
        }

        #[unsafe(method(bannerClicked:))]
        fn banner_clicked(&self, _event: Option<&AnyObject>) {
            self.ivars().state.borrow_mut().banner_clicked(_event);
        }

        #[unsafe(method(screensChanged:))]
        fn screens_changed(&self, _n: Option<&AnyObject>) {
            let mtm = MainThreadMarker::from(self);
            let mut st = self.ivars().state.borrow_mut();
            st.overlay.rebuild(mtm);
        }
    }
);

impl AppTarget {
    fn init(mtm: MainThreadMarker) -> Retained<Self> {
        let state = AppState::new(mtm);
        let this = Self::alloc(mtm).set_ivars(AppIvars {
            state: RefCell::new(state),
        });
        unsafe { msg_send![super(this), init] }
    }
}

pub fn run() {
    let mtm = MainThreadMarker::new().expect("must start on main thread");
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(objc2_app_kit::NSApplicationActivationPolicy::Accessory);

    let target = AppTarget::init(mtm);

    {
        let mut st = target.ivars().state.borrow_mut();
        let tray = Tray::new(mtm, &target);
        tray.set_blink_enabled(st.settings.blink.enabled);
        tray.set_break_enabled(st.settings.eye_break.enabled);
        tray.set_login_enabled(st.settings.behaviour.start_at_login);
        st.sync_pause_ui(mtm);
        st.tray = Some(tray);

        // Repeating 1s schedulers only. High-frequency timers are started on demand.
        let blink_timer = unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                1.0,
                &target,
                sel!(blinkDue:),
                None,
                true,
            )
        };
        let break_t = unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                1.0,
                &target,
                sel!(breakDue:),
                None,
                true,
            )
        };
        st.blink_timer = Some(blink_timer);
        st.break_timer_obj = Some(break_t);
        st.target = Some(target.clone());

        unsafe {
            let center = NSNotificationCenter::defaultCenter();
            let name = NSApplicationDidChangeScreenParametersNotification;
            center.addObserver_selector_name_object(
                &target,
                sel!(screensChanged:),
                Some(name),
                None,
            );
        }
    }

    // Keep target alive for the process lifetime.
    let _keep: Rc<Retained<AppTarget>> = Rc::new(target);
    std::mem::forget(_keep);

    app.run();
}
