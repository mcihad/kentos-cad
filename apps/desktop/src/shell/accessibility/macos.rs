//! Native elements for Iced's painted widgets. The host never participates in mouse hit testing.

use std::cell::RefCell;

use iced::Rectangle;
use iced::futures::channel::mpsc::UnboundedSender;
use objc2::rc::{Retained, Weak};
use objc2::runtime::{AnyObject, Sel};
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, Message as _, define_class, msg_send, sel,
};
use objc2_app_kit::{
    NSAccessibilityAnnouncementKey, NSAccessibilityAnnouncementRequestedNotification,
    NSAccessibilityButtonRole, NSAccessibilityElement,
    NSAccessibilityFocusedUIElementChangedNotification, NSAccessibilityGroupRole,
    NSAccessibilityLayoutChangedNotification, NSAccessibilityPostNotification,
    NSAccessibilityPostNotificationWithUserInfo, NSAccessibilityStaticTextRole,
    NSAccessibilityTextFieldRole, NSAutoresizingMaskOptions, NSView,
};
use objc2_foundation::{NSArray, NSDictionary, NSPoint, NSRect, NSSize, NSString};
use raw_window_handle::RawWindowHandle;

use super::{Control, Event, Node};

type Elements = RefCell<Vec<Retained<Element>>>;

define_class!(
    // SAFETY: The host is a main-thread AppKit view with no keyboard or mouse handlers.
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "KentOSAccessibilityView"]
    #[ivars = Elements]
    struct Host;

    impl Host {
        #[unsafe(method_id(hitTest:))]
        fn hit_test(&self, _point: NSPoint) -> Option<Retained<NSView>> {
            None
        }

        #[unsafe(method(isFlipped))]
        fn flipped(&self) -> bool {
            true
        }

        #[unsafe(method(isAccessibilityElement))]
        fn is_accessible(&self) -> bool {
            !self.ivars().borrow().is_empty()
        }

        #[unsafe(method_id(accessibilityRole))]
        fn role(&self) -> Retained<NSString> {
            // SAFETY: AppKit's immutable role constants live for the process.
            unsafe { NSAccessibilityGroupRole }.retain()
        }

        #[unsafe(method_id(accessibilityLabel))]
        fn label(&self) -> Retained<NSString> {
            NSString::from_str("Mimari çizim araçları")
        }

        #[unsafe(method_id(accessibilityFocusedUIElement))]
        fn focused_element(&self) -> Option<Retained<Element>> {
            self.ivars()
                .borrow()
                .iter()
                .find(|element| element.ivars().data.borrow().0.focused)
                .cloned()
        }

        #[unsafe(method_id(accessibilityChildren))]
        fn children(&self) -> Retained<NSArray<Element>> {
            NSArray::from_retained_slice(&self.ivars().borrow())
        }
    }
);

struct ElementState {
    data: RefCell<(Node, Rectangle)>,
    host: Weak<Host>,
    sender: UnboundedSender<Event>,
}

define_class!(
    // SAFETY: Elements are retained and called on the AppKit main thread. They hold a weak view reference.
    #[unsafe(super(NSAccessibilityElement))]
    #[thread_kind = MainThreadOnly]
    #[name = "KentOSAccessibilityElement"]
    #[ivars = ElementState]
    struct Element;

    impl Element {
        #[unsafe(method(isAccessibilityElement))]
        fn is_accessible(&self) -> bool {
            true
        }

        #[unsafe(method_id(accessibilityRole))]
        fn role(&self) -> Retained<NSString> {
            // SAFETY: AppKit's immutable role constants live for the process.
            let role = unsafe {
                match self.ivars().data.borrow().0.control {
                    Control::ArchTab | Control::Tool(_) => NSAccessibilityButtonRole,
                    Control::Prompt => NSAccessibilityStaticTextRole,
                    Control::Input => NSAccessibilityTextFieldRole,
                }
            };
            role.retain()
        }

        #[unsafe(method_id(accessibilityParent))]
        fn parent(&self) -> Option<Retained<Host>> {
            self.ivars().host.load()
        }

        #[unsafe(method(accessibilityFrame))]
        fn frame(&self) -> NSRect {
            let bounds = self.ivars().data.borrow().1;
            let Some(host) = self.ivars().host.load() else {
                return NSRect::ZERO;
            };
            let Some(window) = host.window() else {
                return NSRect::ZERO;
            };
            let rect = NSRect::new(
                NSPoint::new(f64::from(bounds.x), f64::from(bounds.y)),
                NSSize::new(f64::from(bounds.width), f64::from(bounds.height)),
            );
            window.convertRectToScreen(host.convertRect_toView(rect, None))
        }

        #[unsafe(method_id(accessibilityIdentifier))]
        fn identifier(&self) -> Retained<NSString> {
            NSString::from_str(match self.ivars().data.borrow().0.control {
                Control::ArchTab => "ribbon.arch",
                Control::Tool(tool) => tool.id(),
                Control::Prompt => "commandline.status",
                Control::Input => "commandline.input",
            })
        }

        #[unsafe(method_id(accessibilityLabel))]
        fn label(&self) -> Retained<NSString> {
            NSString::from_str(&self.ivars().data.borrow().0.label)
        }

        #[unsafe(method_id(accessibilityHelp))]
        fn help(&self) -> Retained<NSString> {
            NSString::from_str(&self.ivars().data.borrow().0.hint)
        }

        #[unsafe(method_id(accessibilityValue))]
        fn value(&self) -> Option<Retained<NSString>> {
            let data = self.ivars().data.borrow();
            match data.0.control {
                Control::ArchTab | Control::Tool(_) => None,
                _ => Some(NSString::from_str(&data.0.value)),
            }
        }

        #[unsafe(method(isAccessibilityEnabled))]
        fn enabled(&self) -> bool {
            self.ivars().data.borrow().0.enabled
        }

        #[unsafe(method(isAccessibilityFocused))]
        fn focused(&self) -> bool {
            self.ivars().data.borrow().0.focused
        }

        #[unsafe(method(isAccessibilitySelected))]
        fn selected(&self) -> bool {
            self.ivars().data.borrow().0.selected
        }

        #[unsafe(method(isAccessibilitySelectorAllowed:))]
        fn selector_allowed(&self, selector: Sel) -> bool {
            let node = &self.ivars().data.borrow().0;
            if selector == sel!(accessibilityPerformPress) {
                return (node.enabled && matches!(node.control, Control::ArchTab | Control::Tool(_) | Control::Input)).into();
            }
            if selector == sel!(setAccessibilityValue:) || selector == sel!(setAccessibilityFocused:) {
                return (node.enabled && node.control == Control::Input).into();
            }
            // SAFETY: The other selectors keep the platform's default exposure.
            unsafe { msg_send![super(self), isAccessibilitySelectorAllowed: selector] }
        }

        #[unsafe(method(accessibilityPerformPress))]
        fn press(&self) -> bool {
            let node = &self.ivars().data.borrow().0;
            if !node.enabled {
                return false.into();
            }
            let event = match node.control {
                Control::ArchTab => Event::OpenArchTab,
                Control::Tool(tool) => Event::Activate(tool),
                Control::Input => Event::FocusInput,
                Control::Prompt => return false.into(),
            };
            self.ivars().sender.unbounded_send(event).is_ok()
        }

        #[unsafe(method(setAccessibilityFocused:))]
        fn set_focused(&self, focused: bool) {
            if focused && self.ivars().data.borrow().0.control == Control::Input {
                let _ = self.ivars().sender.unbounded_send(Event::FocusInput);
            }
        }


        #[unsafe(method(setAccessibilityValue:))]
        fn set_value(&self, value: Option<&AnyObject>) {
            if self.ivars().data.borrow().0.control == Control::Input
                && let Some(value) = value.and_then(|value| value.downcast_ref::<NSString>())
            {
                let _ = self.ivars().sender.unbounded_send(Event::SetInput(value.to_string()));
            }
        }

    }
);

pub(super) fn sync(
    window: &dyn iced::window::Window,
    nodes: Vec<(Node, Rectangle)>,
    sender: UnboundedSender<Event>,
    announcement: Option<&str>,
) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let Ok(handle) = window.window_handle() else {
        return;
    };
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return;
    };
    // SAFETY: The live AppKit handle borrows Winit's NSView, and this is the AppKit main thread.
    let view = unsafe { handle.ns_view.cast::<NSView>().as_ref() };
    let existing = view
        .subviews()
        .iter()
        .find_map(|view| view.downcast::<Host>().ok());
    let host = match existing {
        Some(host) => host,
        None if nodes.is_empty() => return,
        None => {
            let host = Host::alloc(mtm).set_ivars(RefCell::default());
            // SAFETY: Initialize our NSView subclass using NSView's designated initializer.
            let host: Retained<Host> =
                unsafe { msg_send![super(host), initWithFrame: view.bounds()] };
            host.setAutoresizingMask(
                NSAutoresizingMaskOptions::ViewWidthSizable
                    | NSAutoresizingMaskOptions::ViewHeightSizable,
            );
            view.addSubview(&host);
            host
        }
    };

    let old = host.ivars().take();
    let mut structure_changed = old.len() != nodes.len();
    let mut focused_element = None;
    let elements: Vec<_> = nodes
        .into_iter()
        .enumerate()
        .map(|(index, (node, bounds))| {
            structure_changed |= old
                .get(index)
                .is_none_or(|old| old.ivars().data.borrow().0.control != node.control);
            if let Some(retained) = old
                .iter()
                .find(|old| old.ivars().data.borrow().0.control == node.control)
            {
                if node.focused && !retained.ivars().data.borrow().0.focused {
                    focused_element = Some(retained.clone());
                }
                retained.ivars().data.replace((node, bounds));
                retained.clone()
            } else {
                let element = Element::alloc(mtm).set_ivars(ElementState {
                    data: RefCell::new((node, bounds)),
                    host: Weak::from_retained(&host),
                    sender: sender.clone(),
                });
                // SAFETY: Initialize our NSAccessibilityElement subclass on the main thread.
                unsafe { msg_send![super(element), init] }
            }
        })
        .collect();
    host.ivars().replace(elements);
    if structure_changed {
        // SAFETY: The host is a live AppKit view; the notification is an immutable AppKit constant.
        unsafe { NSAccessibilityPostNotification(&host, NSAccessibilityLayoutChangedNotification) };
    }
    if let Some(element) = focused_element {
        // SAFETY: The element is a live child of the host when its corresponding field takes focus.
        unsafe {
            NSAccessibilityPostNotification(
                &element,
                NSAccessibilityFocusedUIElementChangedNotification,
            );
        }
    }
    if let Some(text) = announcement
        && let Some(window) = view.window()
    {
        let text = NSString::from_str(text);
        // SAFETY: AppKit accepts a live NSWindow and an NSString under the announcement key.
        unsafe {
            let info = NSDictionary::from_slices(
                &[NSAccessibilityAnnouncementKey],
                &[&*text as &AnyObject],
            );
            NSAccessibilityPostNotificationWithUserInfo(
                &window,
                NSAccessibilityAnnouncementRequestedNotification,
                Some(&info),
            );
        }
    }
}
