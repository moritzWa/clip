//! Run ONLY in a macOS test VM: exercises the production gesture selector.
#[path = "../src/image_zoom.rs"]
mod image_zoom;
use objc2::rc::Retained;
use objc2::runtime::NSObjectProtocol;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSApplication, NSBackingStoreType, NSEvent, NSImageView, NSWindow, NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize};

struct EventData {
    delta: f64,
    location: NSPoint,
}
define_class!(
    #[unsafe(super(NSEvent))]
    #[thread_kind = MainThreadOnly]
    #[ivars = EventData]
    struct TestMagnifyEvent;
    unsafe impl NSObjectProtocol for TestMagnifyEvent {}
    impl TestMagnifyEvent {
        #[unsafe(method(magnification))]
        fn delta(&self) -> f64 { self.ivars().delta }
        #[unsafe(method(locationInWindow))]
        fn location(&self) -> NSPoint { self.ivars().location }
    }
);
fn gesture(delta: f64, mtm: MainThreadMarker) -> Retained<TestMagnifyEvent> {
    let event = TestMagnifyEvent::alloc(mtm).set_ivars(EventData {
        delta,
        location: NSPoint::new(300.0, 200.0),
    });
    unsafe { msg_send![super(event), init] }
}
fn main() {
    assert_eq!(
        std::env::var("CLIP_VM_TEST").as_deref(),
        Ok("1"),
        "VM-only QA program"
    );
    let mtm = MainThreadMarker::new().unwrap();
    let _app = NSApplication::sharedApplication(mtm);
    let frame = NSRect::new(NSPoint::ZERO, NSSize::new(600.0, 400.0));
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(mtm),
            frame,
            NSWindowStyleMask::Borderless,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    unsafe {
        window.setReleasedWhenClosed(false);
    }
    let zoom = image_zoom::ImageZoom::new(mtm);
    let image = NSImageView::initWithFrame(NSImageView::alloc(mtm), frame);
    zoom.setDocumentView(Some(&image));
    window.setContentView(Some(&zoom));
    zoom.resize_document(&image);
    assert!(zoom.allowsMagnification());
    zoom.magnifyWithEvent(&gesture(1.0, mtm));
    assert!((zoom.magnification() - 2.0).abs() < 0.001);
    let zoomed = zoom.contentView().bounds();
    assert!(
        zoomed.size.width < 310.0,
        "magnification must reduce the visible document area"
    );
    zoom.contentView().scrollToPoint(NSPoint::new(100.0, 75.0));
    zoom.reflectScrolledClipView(&zoom.contentView());
    let panned = zoom.contentView().bounds();
    assert!(
        panned.origin.x > 90.0 && panned.origin.y > 65.0,
        "zoomed image must pan"
    );
    zoom.magnifyWithEvent(&gesture(-0.25, mtm));
    assert!((zoom.magnification() - 1.5).abs() < 0.001);
    zoom.magnifyWithEvent(&gesture(100.0, mtm));
    assert_eq!(zoom.magnification(), image_zoom::MAX_ZOOM);
    zoom.fit();
    assert_eq!(zoom.magnification(), 1.0);
    assert_eq!(image.frame().size, zoom.contentSize());
    assert_eq!(zoom.contentView().bounds().origin, NSPoint::ZERO);
    zoom.smartMagnifyWithEvent(&gesture(0.0, mtm));
    assert_eq!(zoom.magnification(), 2.0);
    zoom.smartMagnifyWithEvent(&gesture(0.0, mtm));
    assert_eq!(zoom.magnification(), 1.0);
    zoom.zoom_center(1.5);
    assert_eq!(zoom.magnification(), 1.5);
    zoom.fit();
    println!("PASS: production pinch selector, zoom bounds, native panning, Fit, smart zoom");
    window.orderOut(None);
}
