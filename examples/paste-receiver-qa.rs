//! VM-only native paste target for testing text, image and file batch ordering.
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObjectProtocol};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::*;
use objc2_foundation::{NSPoint, NSRect, NSSize, ns_string};
use std::io::Write;
use std::path::PathBuf;

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[ivars = PathBuf]
    struct PasteReceiver;
    unsafe impl NSObjectProtocol for PasteReceiver {}
    impl PasteReceiver {
        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_focus(&self) -> bool { true }
        #[unsafe(method(paste:))]
        fn paste(&self, _: Option<&AnyObject>) {
            let pb = NSPasteboard::generalPasteboard();
            let record = if let Some(url) = pb.stringForType(unsafe { NSPasteboardTypeFileURL }) {
                format!("file\t{url}\n")
            } else if let Some(image) = pb.dataForType(unsafe { NSPasteboardTypePNG }).or_else(|| pb.dataForType(unsafe { NSPasteboardTypeTIFF })) {
                format!("image\t{} bytes\n", image.length())
            } else {
                format!("text\t{:?}\n", pb.stringForType(unsafe { NSPasteboardTypeString }).map(|s| s.to_string()).unwrap_or_default())
            };
            let mut log = std::fs::OpenOptions::new().append(true).create(true).open(self.ivars()).unwrap();
            log.write_all(record.as_bytes()).unwrap();
        }
    }
);

fn main() {
    assert_eq!(
        std::env::var("CLIP_VM_TEST").as_deref(),
        Ok("1"),
        "VM-only QA program"
    );
    let path: PathBuf = std::env::args()
        .nth(1)
        .expect("path to synthetic paste log")
        .into();
    let mtm = MainThreadMarker::new().unwrap();
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
    let menu = NSMenu::new(mtm);
    let item = NSMenuItem::new(mtm);
    item.setTitle(ns_string!("Edit"));
    let edit = NSMenu::new(mtm);
    unsafe {
        edit.addItemWithTitle_action_keyEquivalent(
            ns_string!("Paste"),
            Some(objc2::sel!(paste:)),
            ns_string!("v"),
        );
    }
    item.setSubmenu(Some(&edit));
    menu.addItem(&item);
    app.setMainMenu(Some(&menu));
    let frame = NSRect::new(NSPoint::new(200.0, 200.0), NSSize::new(480.0, 250.0));
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(mtm),
            frame,
            NSWindowStyleMask::Titled,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    unsafe {
        window.setReleasedWhenClosed(false);
    }
    window.setTitle(ns_string!("Clipglass Synthetic Paste Receiver"));
    let receiver = PasteReceiver::alloc(mtm).set_ivars(path);
    let receiver: Retained<PasteReceiver> =
        unsafe { msg_send![super(receiver), initWithFrame: frame] };
    let label = NSTextField::labelWithString(ns_string!("Synthetic clipboard paste target"), mtm);
    label.setFrame(NSRect::new(
        NSPoint::new(24.0, 190.0),
        NSSize::new(420.0, 24.0),
    ));
    receiver.addSubview(&label);
    window.setContentView(Some(&receiver));
    window.makeKeyAndOrderFront(None);
    window.makeFirstResponder(Some(&receiver));
    app.activateIgnoringOtherApps(true);
    app.run();
}
