#[cfg(target_os = "macos")]
mod macos {
    use std::{cell::RefCell, collections::HashMap, mem, ptr, sync::Once};

    use gpui::Window;
    use objc2::{
        ffi, msg_send,
        rc::Retained,
        runtime::{AnyObject, AnyProtocol, Imp, Sel},
        sel,
    };
    use objc2_foundation::NSString;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    static INSTALL_TEXT_CONTENT: Once = Once::new();

    thread_local! {
        static CONTENT_TYPES: RefCell<HashMap<usize, Retained<NSString>>> =
            RefCell::new(HashMap::new());
    }

    pub fn set_text_content_type(window: &Window, content_type: Option<&str>) {
        let Some(view) = ns_view(window) else {
            return;
        };

        INSTALL_TEXT_CONTENT.call_once(|| install_text_content(view));
        if view
            .class()
            .instance_method(sel!(setContentType:))
            .is_none()
        {
            return;
        }

        let ns_content_type = content_type.map(NSString::from_str);
        let ns_content_type = ns_content_type.as_ref().map_or(ptr::null_mut(), |value| {
            Retained::as_ptr(value).cast_mut().cast::<AnyObject>()
        });

        unsafe {
            let _: () = msg_send![view, setContentType: ns_content_type];
        }
    }

    fn ns_view(window: &Window) -> Option<&AnyObject> {
        let handle = HasWindowHandle::window_handle(window).ok()?;
        let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
            return None;
        };

        Some(unsafe { &*(handle.ns_view.as_ptr() as *const AnyObject) })
    }

    fn install_text_content(view: &AnyObject) {
        let class = view.class();
        let class = class as *const _ as *mut _;

        unsafe {
            let protocol = AnyProtocol::get(c"NSTextContent");
            if let Some(protocol) = protocol {
                ffi::class_addProtocol(class, protocol);
            }

            let content_type_imp: Imp = mem::transmute(
                content_type as unsafe extern "C-unwind" fn(&AnyObject, Sel) -> *mut AnyObject,
            );
            let set_content_type_imp: Imp = mem::transmute(
                set_content_type as unsafe extern "C-unwind" fn(&AnyObject, Sel, *mut AnyObject),
            );

            ffi::class_addMethod(class, sel!(contentType), content_type_imp, c"@@:".as_ptr());
            ffi::class_addMethod(
                class,
                sel!(setContentType:),
                set_content_type_imp,
                c"v@:@".as_ptr(),
            );
        }
    }

    unsafe extern "C-unwind" fn content_type(this: &AnyObject, _: Sel) -> *mut AnyObject {
        let key = this as *const _ as usize;
        CONTENT_TYPES.with(|content_types| {
            content_types
                .borrow()
                .get(&key)
                .map_or(ptr::null_mut(), |value| {
                    Retained::as_ptr(value).cast_mut().cast::<AnyObject>()
                })
        })
    }

    unsafe extern "C-unwind" fn set_content_type(this: &AnyObject, _: Sel, value: *mut AnyObject) {
        let key = this as *const _ as usize;
        CONTENT_TYPES.with(|content_types| {
            let mut content_types = content_types.borrow_mut();
            if value.is_null() {
                content_types.remove(&key);
            } else if let Some(value) = unsafe { Retained::retain(value.cast::<NSString>()) } {
                content_types.insert(key, value);
            }
        });
    }
}

#[cfg(target_os = "macos")]
pub use macos::set_text_content_type;

/// The user's text replacements, or none when the system setting is off. Cached for 2 s,
/// because the input asks on every frame of a focused field.
pub(crate) fn system_text_replacements() -> std::rc::Rc<Vec<(String, String)>> {
    use std::{cell::RefCell, rc::Rc, time::Duration, time::Instant};

    thread_local! {
        static CACHE: RefCell<Option<(Instant, Rc<Vec<(String, String)>>)>> =
            const { RefCell::new(None) };
    }
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some((read_at, items)) = cache.as_ref()
            && read_at.elapsed() < Duration::from_secs(2)
        {
            return items.clone();
        }
        let items = Rc::new(read_text_replacements());
        *cache = Some((Instant::now(), items.clone()));
        items
    })
}

#[cfg(target_os = "macos")]
fn read_text_replacements() -> Vec<(String, String)> {
    use objc2_app_kit::NSSpellChecker;

    if !NSSpellChecker::isAutomaticTextReplacementEnabled() {
        return Vec::new();
    }
    let dictionary = NSSpellChecker::sharedSpellChecker().userReplacementsDictionary();
    let (shortcuts, phrases) = dictionary.to_vecs();
    shortcuts
        .iter()
        .zip(phrases.iter())
        .map(|(shortcut, phrase)| (shortcut.to_string(), phrase.to_string()))
        .collect()
}

#[cfg(not(target_os = "macos"))]
fn read_text_replacements() -> Vec<(String, String)> {
    Vec::new()
}

/// A text replacement the user can see before it applies: the shortcut, its phrase, and the
/// shortcut's bounds in window coordinates.
pub(crate) struct ReplacementCandidate {
    pub shortcut: String,
    pub phrase: String,
    pub bounds: gpui::Bounds<gpui::Pixels>,
}

#[cfg(target_os = "macos")]
pub(crate) use replacement_indicator::{text_replacement_rejected, update_replacement_indicator};

#[cfg(not(target_os = "macos"))]
pub(crate) fn update_replacement_indicator(
    _owner: gpui::EntityId,
    _window: &gpui::Window,
    _candidate: Option<ReplacementCandidate>,
) {
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn text_replacement_rejected(_owner: gpui::EntityId, _shortcut: &str) -> bool {
    false
}

/// The macOS correction bubble under a typed text replacement shortcut, as AppKit text views
/// and Chromium show it. The x in the bubble rejects the replacement for that shortcut.
#[cfg(target_os = "macos")]
mod replacement_indicator {
    use std::{cell::RefCell, ffi::c_void};

    use block2::RcBlock;
    use gpui::{EntityId, Window};
    use objc2::{MainThreadMarker, Message as _, msg_send, runtime::AnyObject};
    use objc2_app_kit::{
        NSApplication, NSCorrectionIndicatorType, NSEventType, NSSpellChecker, NSView,
    };
    use objc2_foundation::{NSArray, NSPoint, NSRect, NSSize, NSString};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    use super::ReplacementCandidate;

    #[derive(Default)]
    struct Indicator {
        /// The input and the shortcut whose bubble shows.
        shown: Option<(EntityId, String)>,
        /// The input and the shortcut whose bubble the user closed; it stays as typed.
        rejected: Option<(EntityId, String)>,
        /// Set while this code dismisses or replaces a bubble, so its callback is no rejection.
        dismissing: bool,
    }

    enum Action {
        Keep,
        Show,
        Dismiss,
    }

    thread_local! {
        static INDICATOR: RefCell<Indicator> = RefCell::new(Indicator::default());
    }

    pub(crate) fn text_replacement_rejected(owner: EntityId, shortcut: &str) -> bool {
        INDICATOR.with(|indicator| {
            indicator
                .borrow()
                .rejected
                .as_ref()
                .is_some_and(|(id, rejected)| *id == owner && rejected == shortcut)
        })
    }

    /// Shows, keeps, or dismisses the bubble for one painted input. An input that does not own
    /// the bubble or the rejection never changes them.
    pub(crate) fn update_replacement_indicator(
        owner: EntityId,
        window: &Window,
        candidate: Option<ReplacementCandidate>,
    ) {
        let shortcut = candidate
            .as_ref()
            .map(|candidate| candidate.shortcut.as_str());
        let action =
            INDICATOR.with(|indicator| {
                let mut indicator = indicator.borrow_mut();
                // A rejection lasts while the same input still ends in the same shortcut.
                if indicator.rejected.as_ref().is_some_and(|(id, rejected)| {
                    *id == owner && Some(rejected.as_str()) != shortcut
                }) {
                    indicator.rejected = None;
                }
                let rejected = indicator
                    .rejected
                    .as_ref()
                    .is_some_and(|(id, _)| *id == owner);
                let owns = indicator.shown.as_ref().is_some_and(|(id, _)| *id == owner);
                let same =
                    owns && indicator.shown.as_ref().map(|(_, shown)| shown.as_str()) == shortcut;
                if same {
                    Action::Keep
                } else if shortcut.is_some() && !rejected {
                    Action::Show
                } else if owns {
                    Action::Dismiss
                } else {
                    Action::Keep
                }
            });
        let view = match action {
            Action::Keep => return,
            Action::Show | Action::Dismiss => match ns_view(window) {
                Some(view) => view,
                None => return,
            },
        };
        match (action, candidate) {
            (Action::Show, Some(candidate)) => show(owner, view, candidate),
            _ => dismiss(view),
        }
    }

    fn ns_view(window: &Window) -> Option<&NSView> {
        let handle = HasWindowHandle::window_handle(window).ok()?;
        let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
            return None;
        };
        Some(unsafe { &*(handle.ns_view.as_ptr() as *const NSView) })
    }

    /// Runs `work` as its own main-queue item. The bubble calls must not run inside a GPUI
    /// paint: `showCorrectionIndicatorOfType:` spins a nested event loop that drains the main
    /// queue, and a GPUI task then finds the app already borrowed and panics.
    fn on_main_queue(work: Box<dyn FnOnce()>) {
        unsafe extern "C" {
            static _dispatch_main_q: c_void;
            fn dispatch_async_f(
                queue: *const c_void,
                context: *mut c_void,
                work: extern "C" fn(*mut c_void),
            );
        }
        extern "C" fn run(context: *mut c_void) {
            let work = unsafe { Box::from_raw(context.cast::<Box<dyn FnOnce()>>()) };
            work();
        }
        let context = Box::into_raw(Box::new(work)).cast::<c_void>();
        unsafe { dispatch_async_f(&raw const _dispatch_main_q, context, run) };
    }

    fn set_dismissing(dismissing: bool) {
        INDICATOR.with(|indicator| indicator.borrow_mut().dismissing = dismissing);
    }

    fn dismiss(view: &NSView) {
        INDICATOR.with(|indicator| indicator.borrow_mut().shown = None);
        let view = view.retain();
        on_main_queue(Box::new(move || {
            set_dismissing(true);
            NSSpellChecker::sharedSpellChecker().dismissCorrectionIndicatorForView(&view);
            set_dismissing(false);
        }));
    }

    /// The bubble closed by a click, not by typing: AppKit gives no other way to tell the x
    /// from a dismissal on a key press.
    fn closed_by_click() -> bool {
        let Some(mtm) = MainThreadMarker::new() else {
            return false;
        };
        NSApplication::sharedApplication(mtm)
            .currentEvent()
            .is_some_and(|event| {
                matches!(
                    event.r#type(),
                    NSEventType::LeftMouseDown | NSEventType::LeftMouseUp
                )
            })
    }

    fn show(owner: EntityId, view: &NSView, candidate: ReplacementCandidate) {
        // GPUI's view is not flipped: AppKit wants the rect from the view's bottom edge.
        let view_bounds: NSRect = unsafe { msg_send![view as &AnyObject, bounds] };
        let bounds = candidate.bounds;
        let height = f64::from(bounds.size.height);
        let rect = NSRect::new(
            NSPoint::new(
                f64::from(bounds.origin.x),
                view_bounds.size.height - f64::from(bounds.origin.y) - height,
            ),
            NSSize::new(f64::from(bounds.size.width), height),
        );
        INDICATOR.with(|indicator| {
            indicator.borrow_mut().shown = Some((owner, candidate.shortcut.clone()));
        });
        let view = view.retain();
        on_main_queue(Box::new(move || {
            let shortcut = candidate.shortcut.clone();
            let completion = RcBlock::new(move |accepted: *mut NSString| {
                let closed_by_click = accepted.is_null() && closed_by_click();
                INDICATOR.with(|indicator| {
                    let mut indicator = indicator.borrow_mut();
                    if indicator.dismissing {
                        return;
                    }
                    if indicator
                        .shown
                        .as_ref()
                        .is_some_and(|(_, shown)| *shown == shortcut)
                    {
                        indicator.shown = None;
                    }
                    if closed_by_click {
                        indicator.rejected = Some((owner, shortcut.clone()));
                    }
                });
            });
            // Showing a new bubble closes the old one and runs its callback; `dismissing` marks it.
            set_dismissing(true);
            NSSpellChecker::sharedSpellChecker()
                .showCorrectionIndicatorOfType_primaryString_alternativeStrings_forStringInRect_view_completionHandler(
                    NSCorrectionIndicatorType::Default,
                    &NSString::from_str(&candidate.phrase),
                    &NSArray::new(),
                    rect,
                    &view,
                    Some(&completion),
                );
            set_dismissing(false);
        }));
    }
}

use gpui::SharedString;

/// Presentation-independent context-menu model produced by the editor.
#[derive(Default)]
pub struct NativeMenu {
    pub items: Vec<NativeMenuItem>,
}

pub enum NativeMenuItem {
    Separator,
    Action {
        label: SharedString,
        disabled: bool,
        action: Box<dyn gpui::Action>,
    },
}

impl NativeMenu {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn menu(self, label: impl Into<SharedString>, action: Box<dyn gpui::Action>) -> Self {
        self.menu_with_disabled(label, false, action)
    }

    pub fn menu_with_disabled(
        mut self,
        label: impl Into<SharedString>,
        disabled: bool,
        action: Box<dyn gpui::Action>,
    ) -> Self {
        self.items.push(NativeMenuItem::Action {
            label: label.into(),
            disabled,
            action,
        });
        self
    }

    pub fn separator(mut self) -> Self {
        self.items.push(NativeMenuItem::Separator);
        self
    }
}
