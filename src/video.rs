//! Muted native video preview. The same player survives compact/full navigation.
use crate::design::{self, rect};
use objc2::rc::Retained;
use objc2::runtime::NSObjectProtocol;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSAccessibility, NSBezierPath, NSButton, NSColor, NSEvent, NSView};
use objc2_av_foundation::{AVPlayer, AVPlayerLayer};
use objc2_core_media::CMTime;
use objc2_foundation::{NSRect, NSString, NSURL};
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};

pub fn is_video(path: &Path) -> bool {
    path.extension().and_then(|s| s.to_str()).is_some_and(|s| {
        matches!(
            s.to_ascii_lowercase().as_str(),
            "mp4" | "mov" | "m4v" | "avi" | "mpeg" | "mpg"
        )
    })
}

pub fn fraction(x: f64, width: f64) -> f64 {
    if !x.is_finite() || !width.is_finite() || width <= 0.0 {
        return 0.0;
    }
    (x / width).clamp(0.0, 1.0)
}

pub struct Ivars {
    player: RefCell<Option<Retained<AVPlayer>>>,
    layer: RefCell<Option<Retained<AVPlayerLayer>>>,
    button: RefCell<Option<Retained<NSButton>>>,
    path: RefCell<Option<PathBuf>>,
    expanded: Cell<bool>,
    dragging: Cell<bool>,
    resume: Cell<bool>,
    button_playing: Cell<Option<bool>>,
}

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Ivars]
    pub struct VideoPreview;
    unsafe impl NSObjectProtocol for VideoPreview {}
    impl VideoPreview {
        #[unsafe(method(mouseDownCanMoveWindow))]
        fn mouse_down_can_move_window(&self) -> bool { false }
        #[unsafe(method(togglePlayback:))]
        fn toggle_action(&self, _: &NSButton) { self.toggle(); }
        #[unsafe(method(drawRect:))]
        fn draw(&self, _: NSRect) {
            if !self.ivars().expanded.get() { return; }
            let width = (self.bounds().size.width - 66.0).max(1.0);
            NSColor::labelColor().colorWithAlphaComponent(0.18).setFill();
            NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(rect(54.0, 17.0, width, 3.0), 1.5, 1.5).fill();
            let progress = fraction(self.position(), self.duration());
            NSColor::labelColor().setFill();
            NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(rect(54.0, 17.0, width * progress, 3.0), 1.5, 1.5).fill();
            NSBezierPath::bezierPathWithOvalInRect(rect(49.0 + width * progress, 13.5, 10.0, 10.0)).fill();
        }
        #[unsafe(method(mouseDown:))]
        fn down(&self, event: &NSEvent) {
            let p = self.convertPoint_fromView(event.locationInWindow(), None);
            if self.ivars().expanded.get() && p.y <= 36.0 && p.x >= 44.0 {
                self.ivars().dragging.set(true);
                self.ivars().resume.set(self.playing());
                self.pause();
                self.seek_at(p.x);
            }
        }
        #[unsafe(method(mouseDragged:))]
        fn drag(&self, event: &NSEvent) {
            if self.ivars().dragging.get() {
                self.seek_at(self.convertPoint_fromView(event.locationInWindow(), None).x);
            }
        }
        #[unsafe(method(mouseUp:))]
        fn up(&self, event: &NSEvent) {
            if self.ivars().dragging.replace(false) {
                self.seek_at(self.convertPoint_fromView(event.locationInWindow(), None).x);
                if self.ivars().resume.get() { self.play(); }
            }
        }
    }
);

impl VideoPreview {
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(Ivars {
            player: RefCell::new(None),
            layer: RefCell::new(None),
            button: RefCell::new(None),
            path: RefCell::new(None),
            expanded: Cell::new(false),
            dragging: Cell::new(false),
            resume: Cell::new(false),
            button_playing: Cell::new(None),
        });
        let this: Retained<Self> =
            unsafe { msg_send![super(this), initWithFrame: rect(0.0, 0.0, 100.0, 100.0)] };
        this.setWantsLayer(true);
        let layer = unsafe { AVPlayerLayer::playerLayerWithPlayer(None) };
        layer.setCornerRadius(10.0);
        layer.setMasksToBounds(true);
        this.layer().unwrap().addSublayer(&layer);
        *this.ivars().layer.borrow_mut() = Some(layer);
        let button = NSButton::initWithFrame(NSButton::alloc(mtm), rect(2.0, 3.0, 32.0, 30.0));
        button.setBordered(false);
        button.setTitle(&NSString::from_str(""));
        button.setImagePosition(objc2_app_kit::NSCellImagePosition::ImageOnly);
        unsafe {
            button.setTarget(Some(&this));
            button.setAction(Some(objc2::sel!(togglePlayback:)));
        }
        this.addSubview(&button);
        *this.ivars().button.borrow_mut() = Some(button);
        this.setHidden(true);
        this
    }

    pub fn load(&self, path: Option<&Path>, autoplay: bool) {
        if self.ivars().path.borrow().as_deref() == path {
            return;
        }
        self.pause();
        let player = path.map(|p| unsafe {
            let url = NSURL::fileURLWithPath(&NSString::from_str(&p.to_string_lossy()));
            let player = AVPlayer::playerWithURL(&url, MainThreadMarker::new().unwrap());
            player.setMuted(true);
            player
        });
        unsafe {
            self.ivars()
                .layer
                .borrow()
                .as_ref()
                .unwrap()
                .setPlayer(player.as_deref());
        }
        *self.ivars().player.borrow_mut() = player;
        *self.ivars().path.borrow_mut() = path.map(Path::to_path_buf);
        if autoplay {
            self.play();
        }
        self.update();
    }

    pub fn layout_video(&self, frame: NSRect, expanded: bool) {
        self.setFrame(frame);
        self.ivars().expanded.set(expanded);
        let bottom = if expanded { 40.0 } else { 0.0 };
        self.ivars().layer.borrow().as_ref().unwrap().setFrame(rect(
            0.0,
            bottom,
            frame.size.width,
            frame.size.height - bottom,
        ));
        self.ivars()
            .button
            .borrow()
            .as_ref()
            .unwrap()
            .setHidden(!expanded);
        self.setNeedsDisplay(true);
    }
    pub fn playing(&self) -> bool {
        self.ivars()
            .player
            .borrow()
            .as_ref()
            .is_some_and(|p| unsafe { p.rate() > 0.0 })
    }
    pub fn position(&self) -> f64 {
        self.ivars()
            .player
            .borrow()
            .as_ref()
            .map_or(0.0, |p| unsafe { p.currentTime().seconds() })
    }
    pub fn duration(&self) -> f64 {
        self.ivars()
            .player
            .borrow()
            .as_ref()
            .and_then(|p| unsafe { p.currentItem() })
            .map_or(0.0, |i| unsafe { i.duration().seconds() })
    }
    pub fn pause(&self) {
        if let Some(p) = self.ivars().player.borrow().as_ref() {
            unsafe {
                p.pause();
            }
        }
    }
    pub fn play(&self) {
        if let Some(p) = self.ivars().player.borrow().as_ref() {
            unsafe {
                p.play();
            }
        }
    }
    pub fn toggle(&self) {
        if self.playing() {
            self.pause();
        } else {
            let duration = self.duration();
            if duration.is_finite() && duration > 0.0 && self.position() >= duration - 0.05 {
                self.seek_at(54.0);
            }
            self.play();
        }
        self.update();
    }
    fn seek_at(&self, x: f64) {
        let duration = self.duration();
        if duration.is_finite() && duration > 0.0 {
            if let Some(p) = self.ivars().player.borrow().as_ref() {
                unsafe {
                    p.seekToTime_toleranceBefore_toleranceAfter(
                        CMTime::with_seconds(
                            fraction(x - 54.0, self.bounds().size.width - 66.0) * duration,
                            600,
                        ),
                        CMTime::with_seconds(0.0, 600),
                        CMTime::with_seconds(0.0, 600),
                    );
                }
            }
        }
        self.setNeedsDisplay(true);
    }
    pub fn update(&self) {
        let playing = self.playing();
        if self.ivars().button_playing.replace(Some(playing)) != Some(playing) {
            if let Some(button) = self.ivars().button.borrow().as_ref() {
                button.setImage(
                    design::symbol(if playing { "pause.fill" } else { "play.fill" }).as_deref(),
                );
                button.setToolTip(Some(&NSString::from_str(if playing {
                    "Pause (Space)"
                } else {
                    "Play (Space)"
                })));
                button.setAccessibilityLabel(Some(&NSString::from_str(if playing {
                    "Pause video"
                } else {
                    "Play video"
                })));
            }
        }
        if self.ivars().expanded.get() {
            self.setNeedsDisplay(true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scrubber_clamps_and_rejects_unready_media() {
        assert_eq!(fraction(50.0, 100.0), 0.5);
        assert_eq!(fraction(-1.0, 100.0), 0.0);
        assert_eq!(fraction(110.0, 100.0), 1.0);
        for width in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert_eq!(fraction(1.0, width), 0.0);
        }
        assert_eq!(fraction(f64::NAN, 1.0), 0.0);
    }
    #[test]
    fn recognizes_video_not_documents_or_images() {
        assert!(is_video(Path::new("demo.MOV")));
        assert!(is_video(Path::new("demo.mp4")));
        assert!(!is_video(Path::new("demo.pdf")));
        assert!(!is_video(Path::new("demo.png")));
    }
}
