// let () = msg_send! is a common pattern for objc
#![allow(clippy::let_unit_value)]

use super::nsstring_to_str;
use super::window::WindowInner;
use crate::connection::ConnectionOps;
use crate::os::macos::app::create_app_delegate;
use crate::screen::{ScreenInfo, Screens};
use crate::spawn::*;
use crate::Appearance;
use cocoa::appkit::{NSApp, NSApplication, NSApplicationActivationPolicyRegular, NSScreen};
use cocoa::base::{id, nil};
use cocoa::foundation::{NSArray, NSInteger};
use objc::runtime::{Object, BOOL, YES};
use objc::*;
use serde::Deserialize;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::rc::Rc;
use std::sync::atomic::AtomicUsize;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub struct Connection {
    ns_app: id,
    pub(crate) windows: RefCell<HashMap<usize, Rc<RefCell<WindowInner>>>>,
    pub(crate) next_window_id: AtomicUsize,
    pub(crate) gl_connection: RefCell<Option<Rc<crate::egl::GlConnection>>>,
}

impl Connection {
    pub(crate) fn create_new() -> anyhow::Result<Self> {
        // Ensure that the SPAWN_QUEUE is created; it will have nothing
        // to run right now.
        SPAWN_QUEUE.run();

        unsafe {
            let ns_app = NSApp();
            ns_app.setActivationPolicy_(NSApplicationActivationPolicyRegular);

            let delegate = create_app_delegate();
            let () = msg_send![ns_app, setDelegate: delegate];

            // Track display topology reconfiguration so the resize logic can
            // avoid clobbering AppKit's restored frame on clamshell sleep/wake.
            // See `is_display_reconfiguring`.
            let err =
                CGDisplayRegisterReconfigurationCallback(reconfig_callback, std::ptr::null_mut());
            log::debug!("registered display reconfiguration callback (err={err})");

            let conn = Self {
                ns_app,
                windows: RefCell::new(HashMap::new()),
                next_window_id: AtomicUsize::new(1),
                gl_connection: RefCell::new(None),
            };
            Ok(conn)
        }
    }

    pub(crate) fn next_window_id(&self) -> usize {
        self.next_window_id
            .fetch_add(1, ::std::sync::atomic::Ordering::Relaxed)
    }

    pub(crate) fn window_by_id(&self, window_id: usize) -> Option<Rc<RefCell<WindowInner>>> {
        self.windows.borrow().get(&window_id).map(Rc::clone)
    }

    pub(crate) fn with_window_inner<
        R,
        F: FnOnce(&mut WindowInner) -> anyhow::Result<R> + Send + 'static,
    >(
        window_id: usize,
        f: F,
    ) -> promise::Future<R>
    where
        R: Send + 'static,
    {
        let mut prom = promise::Promise::new();
        let future = prom.get_future().unwrap();
        promise::spawn::spawn_into_main_thread(async move {
            if let Some(handle) = Connection::get().unwrap().window_by_id(window_id) {
                let mut inner = handle.borrow_mut();
                prom.result(f(&mut inner));
            }
        })
        .detach();

        future
    }
}

type CGDirectDisplayID = u32;
type CGDisplayChangeSummaryFlags = u32;
type CGError = i32;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGDisplayRegisterReconfigurationCallback(
        callback: extern "C" fn(CGDirectDisplayID, CGDisplayChangeSummaryFlags, *mut c_void),
        user_info: *mut c_void,
    ) -> CGError;
}

/// How long after a display reconfiguration callback we keep treating the
/// display topology as "in flux". macOS delivers the wake `Resized` event in the
/// middle of a reconfiguration burst — the leading `Begin` flag fires ~30ms
/// before it, while the `Add`/`Enabled` flags arrive ~15ms after — so we arm
/// this window on *any* callback (catching `Begin`) and hold it open long enough
/// to span the whole burst. An overlong window is harmless; see
/// `is_display_reconfiguring`.
const RECONFIG_WINDOW: Duration = Duration::from_millis(1000);

/// Deadline until which a display reconfiguration is considered in progress.
/// Written by the CoreGraphics reconfiguration callback and read by
/// `wezterm-gui`'s resize logic. `Mutex<Option<Instant>>` keeps the `Instant`
/// native and the static a one-liner (`Mutex::new` is const); the lock is
/// uncontended (written only during a reconfiguration, read only on resize).
static DISPLAY_RECONFIG_DEADLINE: Mutex<Option<Instant>> = Mutex::new(None);

extern "C" fn reconfig_callback(
    _display: CGDirectDisplayID,
    _flags: CGDisplayChangeSummaryFlags,
    _user_info: *mut c_void,
) {
    // Arm on every callback, including the leading `Begin` flag — the earliest
    // marker, and the only one that fires before the decisive wake `Resized`.
    *DISPLAY_RECONFIG_DEADLINE.lock().unwrap() = Some(Instant::now() + RECONFIG_WINDOW);
}

/// Returns true while a display topology reconfiguration (monitor add/remove,
/// mode/resolution change, or sleep/wake teardown) is in flux.
///
/// `wezterm-gui` consults this on a DPI-change resize to decide whether to
/// preserve the existing terminal grid (the normal Retina-style behavior) or to
/// adopt the frame AppKit just restored. During a reconfiguration the grid must
/// NOT be preserved: on clamshell wake, AppKit restores the correct pre-sleep
/// frame, and reapplying a grid captured against the transient phantom display
/// would shrink the window, clobbering that restore. Outside a reconfiguration
/// the two behaviors only differ when the frame actually changed, so a slightly
/// overlong window is harmless.
pub fn is_display_reconfiguring() -> bool {
    match *DISPLAY_RECONFIG_DEADLINE.lock().unwrap() {
        Some(deadline) => Instant::now() < deadline,
        None => false,
    }
}

/// `/System/Library/CoreServices/SystemVersion.plist`
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct SoftwareVersion {
    product_build_version: String,
    product_user_visible_version: String,
    product_name: String,
}

impl SoftwareVersion {
    fn load() -> anyhow::Result<Self> {
        let vers: Self = plist::from_file("/System/Library/CoreServices/SystemVersion.plist")?;
        Ok(vers)
    }
}

impl ConnectionOps for Connection {
    fn name(&self) -> String {
        if let Ok(vers) = SoftwareVersion::load() {
            format!(
                "{} {} ({})",
                vers.product_name, vers.product_user_visible_version, vers.product_build_version
            )
        } else {
            "macOS".to_string()
        }
    }

    fn default_dpi(&self) -> f64 {
        if let Ok(screens) = self.screens() {
            screens.active.effective_dpi.unwrap_or(crate::DEFAULT_DPI)
        } else {
            crate::DEFAULT_DPI
        }
    }

    fn terminate_message_loop(&self) {
        unsafe {
            // bounce via an event callback to encourage stop to apply
            // to the correct level of run loop
            promise::spawn::spawn_into_main_thread(async move {
                let () = msg_send![NSApp(), stop: nil];
                // Generate a UI event so that the run loop breaks out
                // after receiving the stop
                let () = msg_send![NSApp(), abortModal];
            })
            .detach();
        }
    }

    fn get_appearance(&self) -> Appearance {
        let name = unsafe {
            let appearance: id = msg_send![self.ns_app, effectiveAppearance];
            nsstring_to_str(msg_send![appearance, name])
        };
        log::debug!("NSAppearanceName is {name}");
        match name {
            "NSAppearanceNameVibrantDark" | "NSAppearanceNameDarkAqua" => Appearance::Dark,
            "NSAppearanceNameVibrantLight" | "NSAppearanceNameAqua" => Appearance::Light,
            "NSAppearanceNameAccessibilityHighContrastVibrantLight"
            | "NSAppearanceNameAccessibilityHighContrastAqua" => Appearance::LightHighContrast,
            "NSAppearanceNameAccessibilityHighContrastVibrantDark"
            | "NSAppearanceNameAccessibilityHighContrastDarkAqua" => Appearance::DarkHighContrast,
            _ => {
                log::warn!("Unknown NSAppearanceName {name}, assume Light");
                Appearance::Light
            }
        }
    }

    fn run_message_loop(&self) -> anyhow::Result<()> {
        unsafe {
            self.ns_app.run();
        }
        self.windows.borrow_mut().clear();
        Ok(())
    }

    fn hide_application(&self) {
        unsafe {
            let () = msg_send![self.ns_app, hide: self.ns_app];
        }
    }

    fn beep(&self) {
        unsafe {
            NSBeep();
        }
    }

    fn screens(&self) -> anyhow::Result<Screens> {
        let mut by_name = HashMap::new();
        let mut virtual_rect = euclid::rect(0, 0, 0, 0);

        let screens = unsafe { NSScreen::screens(nil) };
        for idx in 0..unsafe { screens.count() } {
            let screen = unsafe { screens.objectAtIndex(idx) };
            let screen = nsscreen_to_screen_info(screen);
            virtual_rect = virtual_rect.union(&screen.rect);
            by_name.insert(screen.name.clone(), screen);
        }

        // The screen with the menu bar is always index 0
        let main = nsscreen_to_screen_info(unsafe { screens.objectAtIndex(0) });

        // The active screen is known as the "main" screen in macOS
        let active = nsscreen_to_screen_info(unsafe { NSScreen::mainScreen(nil) });

        Ok(Screens {
            by_name,
            active,
            main,
            virtual_rect,
        })
    }
}

pub fn nsscreen_to_screen_info(screen: *mut Object) -> ScreenInfo {
    let frame = unsafe { NSScreen::frame(screen) };
    let backing_frame = unsafe { NSScreen::convertRectToBacking_(screen, frame) };
    let rect = euclid::rect(
        backing_frame.origin.x as isize,
        backing_frame.origin.y as isize,
        backing_frame.size.width as isize,
        backing_frame.size.height as isize,
    );
    let has_name: BOOL = unsafe { msg_send!(screen, respondsToSelector: sel!(localizedName)) };
    let name = if has_name == YES {
        unsafe { nsstring_to_str(msg_send!(screen, localizedName)) }.to_string()
    } else {
        format!(
            "{}x{}@{},{}",
            backing_frame.size.width,
            backing_frame.size.height,
            backing_frame.origin.x,
            backing_frame.origin.y
        )
    };

    let has_max_fps: BOOL =
        unsafe { msg_send!(screen, respondsToSelector: sel!(maximumFramesPerSecond)) };
    let max_fps = if has_max_fps == YES {
        let max_fps: NSInteger = unsafe { msg_send!(screen, maximumFramesPerSecond) };
        Some(max_fps as usize)
    } else {
        None
    };

    let scale = backing_frame.size.width / frame.size.width;

    let config = config::configuration();
    let effective_dpi = if let Some(dpi) = config.dpi_by_screen.get(&name).copied() {
        Some(dpi)
    } else if let Some(dpi) = config.dpi {
        Some(dpi)
    } else {
        Some(crate::DEFAULT_DPI * scale)
    };

    ScreenInfo {
        name,
        rect,
        scale,
        max_fps,
        effective_dpi,
    }
}

extern "C" {
    fn NSBeep();
}
