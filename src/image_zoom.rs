//! Image preview backed by AppKit's magnifying and scrolling viewport.
use objc2::rc::Retained;
use objc2::runtime::NSObjectProtocol;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSEvent, NSImageView, NSScrollView, NSScrollerStyle};
use objc2_foundation::{NSPoint, NSRect, NSSize};

pub const MAX_ZOOM: f64 = 16.0;

pub fn next_zoom(current: f64, factor: f64) -> f64 {
    if !factor.is_finite() || factor <= 0.0 {
        return current;
    }
    (current * factor).clamp(1.0, MAX_ZOOM)
}

define_class!(
    #[unsafe(super(NSScrollView))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    pub struct ImageZoom;
    unsafe impl NSObjectProtocol for ImageZoom {}

    impl ImageZoom {
        #[unsafe(method(magnifyWithEvent:))]
        fn pinch(&self, event: &NSEvent) {
            let anchor = self.contentView().convertPoint_fromView(event.locationInWindow(), None);
            self.zoom_at(1.0 + event.magnification(), anchor);
        }

        #[unsafe(method(smartMagnifyWithEvent:))]
        fn smart_zoom(&self, event: &NSEvent) {
            if self.magnification() > 1.01 { self.fit(); } else {
                let anchor = self.contentView().convertPoint_fromView(event.locationInWindow(), None);
                self.zoom_at(2.0, anchor);
            }
        }
    }
);

impl ImageZoom {
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(());
        let this: Retained<Self> = unsafe {
            msg_send![super(this), initWithFrame: NSRect::new(NSPoint::ZERO, NSSize::new(100.0, 100.0))]
        };
        this.setDrawsBackground(false);
        this.contentView().setDrawsBackground(false);
        this.setHasVerticalScroller(false);
        this.setHasHorizontalScroller(false);
        this.setScrollerStyle(NSScrollerStyle::Overlay);
        this.setAutohidesScrollers(true);
        this.setAllowsMagnification(true);
        this.setMinMagnification(1.0);
        this.setMaxMagnification(MAX_ZOOM);
        this
    }

    pub fn zoom_at(&self, factor: f64, anchor: NSPoint) {
        let next = next_zoom(self.magnification(), factor);
        if next <= 1.0 {
            self.fit();
            return;
        }
        self.setHasVerticalScroller(true);
        self.setHasHorizontalScroller(true);
        self.setMagnification_centeredAtPoint(next, anchor);
    }

    pub fn zoom_center(&self, factor: f64) {
        let b = self.contentView().bounds();
        self.zoom_at(
            factor,
            NSPoint::new(
                b.origin.x + b.size.width / 2.0,
                b.origin.y + b.size.height / 2.0,
            ),
        );
    }

    pub fn fit(&self) {
        self.setMagnification(1.0);
        // Always-visible system scrollers otherwise leave a scrollable 15pt
        // gutter even at Fit after returning from a magnified image.
        self.setHasVerticalScroller(false);
        self.setHasHorizontalScroller(false);
        if let Some(document) = self.documentView() {
            document.setFrameSize(self.contentSize());
        }
        self.contentView().scrollToPoint(NSPoint::ZERO);
        self.reflectScrolledClipView(&self.contentView());
    }

    pub fn resize_document(&self, image: &NSImageView) {
        let size = self.frame().size;
        if image.frame().size != size {
            self.fit();
            image.setFrame(NSRect::new(NSPoint::ZERO, size));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zoom_limits_and_invalid_gestures() {
        assert_eq!(next_zoom(1.0, 1.5), 1.5);
        assert_eq!(next_zoom(2.0, 0.75), 1.5);
        assert_eq!(next_zoom(1.0, 0.5), 1.0);
        assert_eq!(next_zoom(15.0, 2.0), MAX_ZOOM);
        assert_eq!(next_zoom(2.0, f64::NAN), 2.0);
        assert_eq!(next_zoom(2.0, -1.0), 2.0);
    }
}
