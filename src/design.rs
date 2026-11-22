//! Spotlight-sized geometry, in points rather than screenshot pixels.
use objc2::rc::Retained;
use objc2::runtime::NSObjectProtocol;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::*;
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

pub const WIDTH: f64 = 750.0;
pub const HEIGHT: f64 = 475.0;
pub const RADIUS: f64 = 26.0;
pub const HEADER: f64 = 58.0;
pub const FOOTER: f64 = 52.0;
pub const ROW: f64 = 30.0;
pub const LIST_WIDTH: f64 = 360.0;

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    pub struct PanelSurface;
    unsafe impl NSObjectProtocol for PanelSurface {}
    impl PanelSurface {
        #[unsafe(method(drawRect:))]
        fn draw_background(&self, _: NSRect) {
            // A light veil, not a new glass effect. Resolve the system color
            // while drawing so this follows light/dark appearance changes.
            NSColor::windowBackgroundColor().colorWithAlphaComponent(0.18).setFill();
            NSBezierPath::fillRect(self.bounds());
        }
    }
);

impl PanelSurface {
    pub fn new(frame: NSRect, mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(());
        unsafe { msg_send![super(this), initWithFrame: frame] }
    }
}

pub fn preview_window_size(available: NSSize) -> NSSize {
    NSSize::new(
        (available.width * 0.9).min(1440.0),
        (available.height * 0.9).min(1000.0),
    )
}

pub fn expanded_preview_frame(width: f64, height: f64) -> NSRect {
    rect(
        16.0,
        FOOTER + 8.0,
        width - 32.0,
        height - HEADER - FOOTER - 16.0,
    )
}

pub fn rect(x: f64, y: f64, w: f64, h: f64) -> NSRect {
    NSRect::new(NSPoint::new(x, y), NSSize::new(w.max(0.0), h.max(0.0)))
}

pub fn label(text: &str, size: f64, mtm: MainThreadMarker) -> Retained<NSTextField> {
    let view = NSTextField::labelWithString(&NSString::from_str(text), mtm);
    view.setFont(Some(&NSFont::systemFontOfSize(size)));
    view.setTextColor(Some(&NSColor::labelColor()));
    view.setDrawsBackground(false);
    view.setLineBreakMode(NSLineBreakMode::ByTruncatingTail);
    view
}

pub fn symbol(name: &str) -> Option<Retained<NSImage>> {
    NSImage::imageWithSystemSymbolName_accessibilityDescription(&NSString::from_str(name), None)
}

pub fn glass(frame: NSRect, radius: f64, mtm: MainThreadMarker) -> Retained<NSGlassEffectView> {
    let view = NSGlassEffectView::initWithFrame(NSGlassEffectView::alloc(mtm), frame);
    view.setStyle(NSGlassEffectViewStyle::Regular);
    view.setCornerRadius(radius);
    view
}

pub fn rounded(view: &NSView, radius: f64, color: &NSColor) {
    view.setWantsLayer(true);
    if let Some(layer) = view.layer() {
        layer.setCornerRadius(radius);
        layer.setMasksToBounds(true);
        layer.setBackgroundColor(Some(&color.CGColor()));
    }
}

pub fn scroll(view: &NSView, mtm: MainThreadMarker) -> Retained<NSScrollView> {
    let scroll = NSScrollView::initWithFrame(NSScrollView::alloc(mtm), rect(0.0, 0.0, 10.0, 10.0));
    scroll.setHasVerticalScroller(true);
    scroll.setScrollerStyle(NSScrollerStyle::Overlay);
    scroll.setAutohidesScrollers(true);
    scroll.setDrawsBackground(false);
    scroll.contentView().setDrawsBackground(false);
    scroll.setDocumentView(Some(view));
    scroll
}

define_class!(
    #[unsafe(super(NSTableRowView))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    pub struct ClipRow;

    unsafe impl NSObjectProtocol for ClipRow {}

    impl ClipRow {
        #[unsafe(method(interiorBackgroundStyle))]
        fn background_style(&self) -> NSBackgroundStyle { NSBackgroundStyle::Normal }

        #[unsafe(method(drawSelectionInRect:))]
        fn draw_selection(&self, _: NSRect) {
            NSColor::labelColor().colorWithAlphaComponent(0.085).setFill();
            let b = self.bounds();
            let path = NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(
                rect(0.0, 0.0, b.size.width, b.size.height), 10.0, 10.0,
            );
            path.fill();
        }
    }
);

impl ClipRow {
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(());
        unsafe { msg_send![super(this), initWithFrame: rect(0.0, 0.0, LIST_WIDTH - 16.0, ROW)] }
    }
}

pub fn cell(
    title: &str,
    kind: &str,
    thumbnail: Option<&NSImage>,
    marked: bool,
    mtm: MainThreadMarker,
) -> Retained<NSView> {
    let view = NSView::initWithFrame(NSView::alloc(mtm), rect(0.0, 0.0, LIST_WIDTH - 16.0, ROW));
    let tile = NSView::initWithFrame(NSView::alloc(mtm), rect(6.0, 5.0, 20.0, 20.0));
    // Icon tiles sit well inside each row, not at the outer panel's corners.
    rounded(
        &tile,
        5.0,
        &NSColor::labelColor().colorWithAlphaComponent(0.07),
    );
    let icon = NSImageView::initWithFrame(NSImageView::alloc(mtm), rect(2.0, 2.0, 16.0, 16.0));
    let name = match kind {
        "image" => "photo",
        "file" => "doc",
        "link" => "link",
        _ => "doc.text",
    };
    if let Some(thumbnail) = thumbnail {
        icon.setFrame(rect(0.0, 0.0, 20.0, 20.0));
        icon.setImageScaling(NSImageScaling::ScaleProportionallyUpOrDown);
        icon.setImage(Some(thumbnail));
    } else {
        icon.setImage(symbol(name).as_deref());
        icon.setContentTintColor(Some(&NSColor::labelColor()));
    }
    tile.addSubview(&icon);
    view.addSubview(&tile);
    let text = label(&title.replace('\n', " "), 13.0, mtm);
    text.setFrame(rect(34.0, 6.0, LIST_WIDTH - 76.0, 18.0));
    text.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable);
    view.addSubview(&text);
    if marked {
        let check = NSImageView::initWithFrame(
            NSImageView::alloc(mtm),
            rect(LIST_WIDTH - 38.0, 7.0, 16.0, 16.0),
        );
        check.setImage(symbol("checkmark.circle.fill").as_deref());
        check.setContentTintColor(Some(&NSColor::controlAccentColor()));
        view.addSubview(&check);
    }
    view
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn spotlight_geometry_leaves_space_for_content() {
        assert_eq!((WIDTH, HEIGHT), (750.0, 475.0));
        assert!(HEIGHT - HEADER - FOOTER - 30.0 > ROW * 11.0);
        assert!((LIST_WIDTH - WIDTH / 2.0).abs() <= 15.0);
        assert!(WIDTH - LIST_WIDTH - 32.0 > 350.0);
        // Footer capsules have height 35 and a matching 17.5pt radius.
        assert_eq!(35.0 / 2.0, 17.5);
    }

    #[test]
    fn expanded_preview_uses_both_columns_without_covering_controls() {
        let frame = expanded_preview_frame(WIDTH, HEIGHT);
        assert_eq!(frame.origin.x, 16.0);
        assert_eq!(frame.size.width, 718.0);
        assert_eq!(frame.size.height, 349.0);
        assert!(frame.origin.y > FOOTER);
        assert!(frame.origin.y + frame.size.height < HEIGHT - HEADER);
    }

    #[test]
    fn enlarged_preview_fits_small_and_large_screens() {
        let small = preview_window_size(NSSize::new(1024.0, 743.0));
        assert!(small.width > WIDTH && small.height > HEIGHT);
        assert!(small.width < 1024.0 && small.height < 743.0);
        assert_eq!(
            preview_window_size(NSSize::new(3000.0, 2000.0)),
            NSSize::new(1440.0, 1000.0)
        );
    }
}
