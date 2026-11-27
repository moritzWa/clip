//! Run only in the muted macOS test VM, with a generated 20-second video.
#[path = "../src/design.rs"]
mod design;
#[path = "../src/video.rs"]
mod video;
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSApplication, NSBackingStoreType, NSEvent, NSEventModifierFlags, NSEventType, NSWindow,
    NSWindowStyleMask,
};
use objc2_foundation::{NSDate, NSPoint, NSRunLoop};
use std::time::{Duration, Instant};

fn wait_for(label: &str, mut condition: impl FnMut() -> bool) {
    let end = Instant::now() + Duration::from_secs(10);
    while !condition() {
        assert!(Instant::now() < end, "Timed out: {label}");
        NSRunLoop::currentRunLoop().runUntilDate(&NSDate::dateWithTimeIntervalSinceNow(0.025));
    }
}

fn mouse(
    view: &video::VideoPreview,
    window: &NSWindow,
    x: f64,
    kind: NSEventType,
    _mtm: MainThreadMarker,
) {
    let event = NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
        kind, NSPoint::new(x, 18.0), NSEventModifierFlags::empty(), 0.0, window.windowNumber(), None, 1, 1, 1.0).unwrap();
    match kind {
        NSEventType::LeftMouseDown => view.mouseDown(&event),
        NSEventType::LeftMouseDragged => view.mouseDragged(&event),
        _ => view.mouseUp(&event),
    }
}

fn main() {
    assert_eq!(
        std::env::var("CLIP_VM_TEST").as_deref(),
        Ok("1"),
        "VM-only QA"
    );
    let path = std::env::args().nth(1).expect("synthetic video path");
    let mtm = MainThreadMarker::new().unwrap();
    let _app = NSApplication::sharedApplication(mtm);
    let frame = design::rect(0.0, 0.0, 600.0, 400.0);
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
    let view = video::VideoPreview::new(mtm);
    assert!(!view.mouseDownCanMoveWindow(), "scrubbing must not drag the panel");
    window.setContentView(Some(&view));
    view.setHidden(false);
    view.layout_video(frame, false);
    window.makeKeyAndOrderFront(None);
    view.load(Some(std::path::Path::new(&path)), true);
    wait_for("compact autoplay", || view.position() > 0.3);
    let position = view.position();
    view.layout_video(frame, true);
    assert!(view.position() >= position, "expansion reset position");
    view.toggle();
    assert!(!view.playing());
    mouse(
        &view,
        &window,
        54.0 + 534.0 * 0.5,
        NSEventType::LeftMouseDown,
        mtm,
    );
    mouse(
        &view,
        &window,
        54.0 + 534.0 * 0.5,
        NSEventType::LeftMouseUp,
        mtm,
    );
    wait_for("click-to-seek", || (view.position() - 10.0).abs() < 0.1);
    assert!(!view.playing(), "paused seek must stay paused");
    view.toggle();
    assert!(view.playing());
    mouse(&view, &window, 100.0, NSEventType::LeftMouseDown, mtm);
    mouse(
        &view,
        &window,
        54.0 + 534.0 * 0.75,
        NSEventType::LeftMouseDragged,
        mtm,
    );
    mouse(
        &view,
        &window,
        54.0 + 534.0 * 0.75,
        NSEventType::LeftMouseUp,
        mtm,
    );
    wait_for("drag-to-seek", || (15.0..16.0).contains(&view.position()));
    assert!(view.playing(), "playing scrub must resume");
    view.pause();
    mouse(&view, &window, 1000.0, NSEventType::LeftMouseDown, mtm);
    mouse(&view, &window, 1000.0, NSEventType::LeftMouseUp, mtm);
    wait_for("end seek", || view.position() >= 19.9);
    view.toggle();
    wait_for("replay from end", || {
        view.position() < 1.0 && view.playing()
    });
    view.load(None, false);
    assert!(!view.playing());
    window.orderOut(None);
    println!(
        "PASS: compact autoplay, expansion continuity, toggle, click seek, paused seek, drag seek, resume, clamping, replay, unload"
    );
}
