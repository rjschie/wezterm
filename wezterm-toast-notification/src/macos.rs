#![cfg(target_os = "macos")]
use crate::ToastNotification;
use block2::{Block, RcBlock};
use objc2::rc::Retained;
use objc2::runtime::{Bool, NSObject, NSObjectProtocol, ProtocolObject};
use objc2::{define_class, msg_send, AllocAnyThread};
use objc2_foundation::{ns_string, NSArray, NSDictionary, NSError, NSSet, NSString};
use objc2_user_notifications::{
    UNAuthorizationOptions, UNMutableNotificationContent, UNNotification, UNNotificationAction,
    UNNotificationActionOptions, UNNotificationCategory, UNNotificationCategoryOptions,
    UNNotificationPresentationOptions, UNNotificationRequest, UNNotificationResponse,
    UNNotificationSound, UNUserNotificationCenter, UNUserNotificationCenterDelegate,
};
use std::sync::{LazyLock, Mutex, Once};

const NEEDS_SIGN: &str = "Note that the application must be code-signed \
                          for UNUserNotificationCenter to work";

fn ns_error_to_string(err: *mut NSError) -> String {
    if err.is_null() {
        "null error".to_string()
    } else {
        unsafe {
            let err: &NSError = &*err;
            format!(
                "{} {:?}",
                err.localizedDescription(),
                err.localizedFailureReason()
            )
        }
    }
}

define_class!(
    #[unsafe(super = NSObject)]
    #[name = "WezTermNotifDelegate"]
    #[derive(Debug)]
    struct NotifDelegate;

    unsafe impl NSObjectProtocol for NotifDelegate {}
    unsafe impl UNUserNotificationCenterDelegate for NotifDelegate {
        #[unsafe(method(userNotificationCenter:willPresentNotification:withCompletionHandler:))]
        unsafe fn will_present(
            &self,
            _center: &UNUserNotificationCenter,
            _notification: &UNNotification,
            completion_handler: &block2::Block<dyn Fn(UNNotificationPresentationOptions)>,
        ) {
            log::debug!("will_present");
            let options = UNNotificationPresentationOptions::List
                | UNNotificationPresentationOptions::Sound
                | UNNotificationPresentationOptions::Badge
                | UNNotificationPresentationOptions::Banner;
            completion_handler.call((options,));
        }

        #[unsafe(method(userNotificationCenter:didReceiveNotificationResponse:withCompletionHandler:))]
        unsafe fn did_receive_notification(
            &self,
            _center: &UNUserNotificationCenter,
            response: &UNNotificationResponse,
            completion_handler: &Block<dyn Fn()>,
        ) {
            let action = response.actionIdentifier();
            let user_info = response.notification().request().content().userInfo();
            let url = user_info.valueForKey(ns_string!("url"));

            log::debug!("did_receive_notification -> action={action:?} url={url:?}");

            if let Some(url) = url {
                if let Ok(url_str) = url.downcast::<NSString>() {
                    wezterm_open_url::open_url(&url_str.to_string());
                }
            }

            completion_handler.call(());
        }
    }
);

impl NotifDelegate {
    fn new() -> Retained<Self> {
        let this = Self::alloc().set_ivars(());
        let me: Retained<Self> = unsafe { msg_send![super(this), init] };
        log::debug!("new delegate {:?}", Retained::as_ptr(&me));
        me
    }
}

impl Drop for NotifDelegate {
    fn drop(&mut self) {
        log::debug!("dropping NotifDelegate {:?}", self as *mut Self);
    }
}

const CENTER: LazyLock<Retained<UNUserNotificationCenter>> =
    LazyLock::new(|| unsafe { UNUserNotificationCenter::currentNotificationCenter() });

struct NotifRecord {
    uuid: String,
    pane_id: usize,
    tab_id: usize,
    window_id: usize,
}

/// (pane_id, tab_id, window_id) — set before calling show_notif, consumed inside
static PENDING_CONTEXT: Mutex<Option<(usize, usize, usize)>> = Mutex::new(None);
static NOTIF_RECORDS: Mutex<Vec<NotifRecord>> = Mutex::new(Vec::new());

/// Set the pane/tab/window context for the next notification.
/// Must be called immediately before `show_notif`.
pub fn set_notification_context(pane_id: usize, tab_id: usize, window_id: usize) {
    *PENDING_CONTEXT.lock().unwrap() = Some((pane_id, tab_id, window_id));
}

fn remove_delivered(uuids: Vec<String>) {
    if uuids.is_empty() {
        return;
    }
    let ns_ids: Vec<Retained<NSString>> = uuids.iter().map(|s| NSString::from_str(s)).collect();
    let refs: Vec<&NSString> = ns_ids.iter().map(|s| &**s).collect();
    let array = NSArray::from_slice(&refs);
    CENTER.removeDeliveredNotificationsWithIdentifiers(&array);
}

pub fn dismiss_for_pane(pane_id: usize) {
    let mut records = NOTIF_RECORDS.lock().unwrap();
    let (to_remove, remaining): (Vec<_>, Vec<_>) =
        records.drain(..).partition(|r| r.pane_id == pane_id);
    *records = remaining;
    let uuids: Vec<String> = to_remove.into_iter().map(|r| r.uuid).collect();
    drop(records);
    remove_delivered(uuids);
}

pub fn dismiss_for_tab(tab_id: usize) {
    let mut records = NOTIF_RECORDS.lock().unwrap();
    let (to_remove, remaining): (Vec<_>, Vec<_>) =
        records.drain(..).partition(|r| r.tab_id == tab_id);
    *records = remaining;
    let uuids: Vec<String> = to_remove.into_iter().map(|r| r.uuid).collect();
    drop(records);
    remove_delivered(uuids);
}

pub fn dismiss_for_window(window_id: usize) {
    let mut records = NOTIF_RECORDS.lock().unwrap();
    let (to_remove, remaining): (Vec<_>, Vec<_>) =
        records.drain(..).partition(|r| r.window_id == window_id);
    *records = remaining;
    let uuids: Vec<String> = to_remove.into_iter().map(|r| r.uuid).collect();
    drop(records);
    remove_delivered(uuids);
}

pub fn dismiss_all() {
    NOTIF_RECORDS.lock().unwrap().clear();
    CENTER.removeAllDeliveredNotifications();
}

pub fn initialize() {
    static INIT: Once = Once::new();
    INIT.call_once(|| unsafe {
        CENTER.requestAuthorizationWithOptions_completionHandler(
            UNAuthorizationOptions::Alert
                | UNAuthorizationOptions::Provisional
                | UNAuthorizationOptions::Sound,
            &RcBlock::new(|ok: Bool, err| {
                if ok.is_false() {
                    log::error!(
                        "requestAuthorization status={ok:?} {}. {NEEDS_SIGN}",
                        ns_error_to_string(err)
                    );
                }
            }),
        );

        let show_url = UNNotificationAction::actionWithIdentifier_title_options(
            ns_string!("SHOW_URL"),
            ns_string!("Show"),
            UNNotificationActionOptions::empty(),
        );
        let show_url_cat =
            UNNotificationCategory::categoryWithIdentifier_actions_intentIdentifiers_options(
                ns_string!("SHOW_URL_ACTION"),
                &NSArray::from_retained_slice(&[show_url]),
                &NSArray::from_slice(&[]),
                UNNotificationCategoryOptions::CustomDismissAction,
            );
        CENTER.setNotificationCategories(&NSSet::from_retained_slice(&[show_url_cat]));

        let delegate = NotifDelegate::new();
        let delegate_proto = ProtocolObject::from_retained(delegate.clone());
        CENTER.setDelegate(Some(&delegate_proto));
        log::debug!(
            "after setDelegate {:?}, center.delegate={:?}",
            delegate,
            CENTER.delegate()
        );

        // Intentionally "leak" the delegate.
        // I've tried stashing it into a global to keep it alive,
        // but something still manages to drop the underlying delegate
        // and that will break the weak ref in the center.
        // This is likely not the right way to do this, but after
        // spending two hours scratching my head, this is the least
        // crazy thing.
        Retained::into_raw(delegate);
    });
}

pub fn show_notif(toast: ToastNotification) -> Result<(), Box<dyn std::error::Error>> {
    initialize();
    unsafe {
        log::debug!("show_notif center.delegate is {:?}", CENTER.delegate());

        let notif = UNMutableNotificationContent::new();
        notif.setTitle(&NSString::from_str(&toast.title));
        notif.setBody(&NSString::from_str(&toast.message));
        if toast.sound {
            notif.setSound(Some(&UNNotificationSound::defaultSound()));
        }

        if let Some(url) = &toast.url {
            let info =
                NSDictionary::from_slices(&[ns_string!("url")], &[&*NSString::from_str(&url)]);
            notif.setUserInfo(
                info.downcast_ref::<NSDictionary>()
                    .expect("is NSDictionary"),
            );
            notif.setCategoryIdentifier(ns_string!("SHOW_URL_ACTION"));
        }

        let identifier = uuid::Uuid::new_v4().to_string();

        // Record pane/tab/window context if set by caller
        let ctx = PENDING_CONTEXT.lock().unwrap().take();
        if let Some((pane_id, tab_id, window_id)) = ctx {
            NOTIF_RECORDS.lock().unwrap().push(NotifRecord {
                uuid: identifier.clone(),
                pane_id,
                tab_id,
                window_id,
            });
        }

        let request = UNNotificationRequest::requestWithIdentifier_content_trigger(
            &NSString::from_str(&identifier),
            &*notif,
            None,
        );

        CENTER.addNotificationRequest_withCompletionHandler(
            &*request,
            Some(&RcBlock::new(move |err: *mut NSError| {
                if err.is_null() {
                    if let Some(timeout) = toast.timeout {
                        let identifier = identifier.clone();
                        std::thread::spawn(move || {
                            std::thread::sleep(timeout);
                            // Remove from records
                            NOTIF_RECORDS
                                .lock()
                                .unwrap()
                                .retain(|r| r.uuid != identifier);
                            let ident_array =
                                NSArray::from_retained_slice(&[NSString::from_str(&identifier)]);
                            CENTER.removeDeliveredNotificationsWithIdentifiers(&ident_array);
                        });
                    }
                } else {
                    log::error!("notif failed {}. {NEEDS_SIGN}", ns_error_to_string(err));
                }
            })),
        );
    }

    Ok(())
}
