//! Opaque VM-only backdrop and native paste destination for synthetic demos.
//! Never use this to record a personal desktop or clipboard.
use objc2::rc::Retained;
use objc2::runtime::NSObjectProtocol;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::*;
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString, ns_string};

fn rect(x: f64, y: f64, w: f64, h: f64) -> NSRect {
    NSRect::new(NSPoint::new(x, y), NSSize::new(w, h))
}

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    struct DemoStage;
    unsafe impl NSObjectProtocol for DemoStage {}
    impl DemoStage {
        #[unsafe(method(drawRect:))]
        fn draw_background(&self, _: NSRect) {
            NSColor::colorWithSRGBRed_green_blue_alpha(0.91, 0.93, 0.96, 1.0).setFill();
            NSBezierPath::fillRect(self.bounds());
        }
    }
);

define_class!(
    #[unsafe(super(NSWindow))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    struct DemoWindow;
    unsafe impl NSObjectProtocol for DemoWindow {}
    impl DemoWindow {
        #[unsafe(method(canBecomeKeyWindow))]
        fn key_window(&self) -> bool { true }
        #[unsafe(method(canBecomeMainWindow))]
        fn main_window(&self) -> bool { true }
    }
);

fn label(text: &str, font: f64, frame: NSRect, mtm: MainThreadMarker) -> Retained<NSTextField> {
    let field = NSTextField::labelWithString(&NSString::from_str(text), mtm);
    field.setFont(Some(&NSFont::systemFontOfSize(font)));
    field.setTextColor(Some(&NSColor::colorWithWhite_alpha(0.20, 1.0)));
    field.setFrame(frame);
    field
}

fn main() {
    assert_eq!(
        std::env::var("CLIP_VM_TEST").as_deref(),
        Ok("1"),
        "VM-only demo stage"
    );
    let mtm = MainThreadMarker::new().unwrap();
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
    let menu = NSMenu::new(mtm);
    let app_item = NSMenuItem::new(mtm);
    let app_menu = NSMenu::new(mtm);
    unsafe {
        app_menu.addItemWithTitle_action_keyEquivalent(
            ns_string!("Quit Demo Stage"),
            Some(objc2::sel!(terminate:)),
            ns_string!("q"),
        );
    }
    app_item.setSubmenu(Some(&app_menu));
    menu.addItem(&app_item);
    let edit_item = NSMenuItem::new(mtm);
    edit_item.setTitle(ns_string!("Edit"));
    let edit = NSMenu::new(mtm);
    for (name, action, key) in [
        ("Paste", objc2::sel!(paste:), "v"),
        ("Select All", objc2::sel!(selectAll:), "a"),
    ] {
        unsafe {
            edit.addItemWithTitle_action_keyEquivalent(
                &NSString::from_str(name),
                Some(action),
                &NSString::from_str(key),
            );
        }
    }
    edit_item.setSubmenu(Some(&edit));
    menu.addItem(&edit_item);
    app.setMainMenu(Some(&menu));
    let screen = NSScreen::mainScreen(mtm).unwrap().visibleFrame();
    let window = DemoWindow::alloc(mtm).set_ivars(());
    let window: Retained<DemoWindow> = unsafe {
        msg_send![super(window), initWithContentRect: screen, styleMask: NSWindowStyleMask::Borderless, backing: NSBackingStoreType::Buffered, defer: false]
    };
    unsafe {
        window.setReleasedWhenClosed(false);
    }
    window.setTitle(ns_string!("Clipglass Demo Stage"));
    let stage = DemoStage::alloc(mtm).set_ivars(());
    let stage: Retained<DemoStage> = unsafe {
        msg_send![super(stage), initWithFrame: rect(0.0,0.0,screen.size.width,screen.size.height)]
    };
    let w = screen.size.width;
    let h = screen.size.height;
    let title = label("Clipglass", 34.0, rect(48.0, h - 72.0, w - 96.0, 42.0), mtm);
    stage.addSubview(&title);
    let subtitle = label(
        "Search. Preview. Paste.",
        15.0,
        rect(50.0, h - 99.0, w - 100.0, 24.0),
        mtm,
    );
    stage.addSubview(&subtitle);
    let badge = label(
        "SYNTHETIC DEMO",
        11.0,
        rect(w - 210.0, h - 58.0, 162.0, 20.0),
        mtm,
    );
    badge.setAlignment(NSTextAlignment::Right);
    stage.addSubview(&badge);
    stage.addSubview(&label(
        "Paste destination",
        14.0,
        rect(52.0, 192.0, w - 104.0, 22.0),
        mtm,
    ));
    let text = NSTextView::initWithFrame(NSTextView::alloc(mtm), rect(0.0, 0.0, w - 104.0, 140.0));
    text.setRichText(false);
    text.setEditable(true);
    text.setSelectable(true);
    text.setFont(Some(&NSFont::monospacedSystemFontOfSize_weight(
        17.0,
        unsafe { NSFontWeightRegular },
    )));
    text.setTextColor(Some(&NSColor::colorWithWhite_alpha(0.16, 1.0)));
    text.setBackgroundColor(&NSColor::whiteColor());
    text.setTextContainerInset(NSSize::new(18.0, 16.0));
    let scroll =
        NSScrollView::initWithFrame(NSScrollView::alloc(mtm), rect(48.0, 44.0, w - 96.0, 140.0));
    scroll.setDocumentView(Some(&text));
    scroll.setWantsLayer(true);
    if let Some(layer) = scroll.layer() {
        layer.setCornerRadius(16.0);
        layer.setMasksToBounds(true);
    }
    stage.addSubview(&scroll);
    stage.addSubview(&label(
        "Real app  ·  Generated clips only  ·  No audio",
        11.0,
        rect(52.0, 14.0, w - 104.0, 18.0),
        mtm,
    ));
    window.setContentView(Some(&stage));
    window.makeKeyAndOrderFront(None);
    window.makeFirstResponder(Some(&text));
    app.activateIgnoringOtherApps(true);
    app.run();
}
