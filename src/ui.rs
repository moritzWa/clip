use std::cell::{Cell, RefCell};
use std::collections::{HashSet, VecDeque};
use std::rc::Rc;
use std::time::{Duration, Instant};

use block2::RcBlock;
use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObjectProtocol, ProtocolObject};
use objc2::{AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationOptions, NSApplicationActivationPolicy,
    NSBackingStoreType, NSButton, NSControlTextEditingDelegate, NSEvent, NSEventMask,
    NSGlassEffectView, NSImage, NSImageView, NSPasteboard, NSPasteboardTypeFileURL,
    NSPasteboardTypePNG, NSPasteboardTypeString, NSPasteboardTypeTIFF, NSPopUpButton,
    NSRunningApplication, NSScreen, NSScrollView, NSTableColumn, NSTableView,
    NSTableViewDataSource, NSTableViewDelegate, NSTextField, NSTextView, NSView, NSWindow,
    NSWindowDelegate, NSWindowStyleMask, NSWorkspace,
};
use objc2_foundation::NSURL;
use objc2_foundation::{
    NSIndexSet, NSInteger, NSPoint, NSRect, NSSize, NSString, NSTimer, ns_string,
};
use objc2_quick_look_ui::{QLPreviewView, QLPreviewViewStyle};

use crate::design::{self, rect};
use crate::paste;
use crate::store::{self, Hit, Store};

struct Ivars {
    shortcuts: Rc<RefCell<crate::shortcuts::Service>>,
    settings: RefCell<Option<Retained<crate::settings::SettingsWindow>>>,
    status_item: RefCell<Option<Retained<objc2_app_kit::NSStatusItem>>>,
    store: RefCell<Store>,
    rows: RefCell<Vec<Hit>>,
    recent: RefCell<Vec<Hit>>,
    thumbnails: RefCell<crate::thumbnails::Thumbnails>,
    marked: RefCell<HashSet<String>>,
    paste_queue: RefCell<VecDeque<store::Preview>>,
    paste_timer: RefCell<Option<Retained<NSTimer>>>,
    pasting: Cell<bool>,
    query: RefCell<String>,
    search: RefCell<Option<Retained<NSTextField>>>,
    preview_title: RefCell<Option<Retained<NSTextField>>>,
    surface: RefCell<Option<Retained<NSView>>>,
    glass: RefCell<Option<Retained<NSGlassEffectView>>>,
    filter: RefCell<Option<Retained<NSPopUpButton>>>,
    divider: RefCell<Option<Retained<NSView>>>,
    info: RefCell<
        Vec<(
            Retained<NSView>,
            Retained<NSTextField>,
            Retained<NSTextField>,
        )>,
    >,
    filter_index: Cell<isize>,
    table: RefCell<Option<Retained<NSTableView>>>,
    text: RefCell<Option<Retained<NSTextView>>>,
    image: RefCell<Option<Retained<NSImageView>>>,
    zoom_image: RefCell<Option<Retained<NSImageView>>>,
    image_zoom: RefCell<Option<Retained<crate::image_zoom::ImageZoom>>>,
    zoom_buttons: RefCell<Vec<Retained<NSButton>>>,
    zoom_status: RefCell<Option<Retained<NSTextField>>>,
    document: RefCell<Option<Retained<QLPreviewView>>>,
    preview_kind: Cell<u8>,
    video: RefCell<Option<Retained<crate::video::VideoPreview>>>,
    meta: RefCell<Option<Retained<NSTextField>>>,
    list_scroll: RefCell<Option<Retained<NSScrollView>>>,
    fade_state: Cell<(i32, i32)>,
    text_scroll: RefCell<Option<Retained<NSScrollView>>>,
    capture: RefCell<crate::clipboard::Capture>,
    ignore_change: Cell<isize>,
    prev_pid: Cell<i32>,
    previewing: Cell<bool>,
    ocr: crate::ocr::Worker,
    ocr_pending: RefCell<HashSet<String>>,
    ocr_scan_at: Cell<Instant>,
}

define_class!(
    #[unsafe(super(NSWindow))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Ivars]
    struct ClipPanel;

    unsafe impl NSObjectProtocol for ClipPanel {}
    unsafe impl NSControlTextEditingDelegate for ClipPanel {}

    unsafe impl NSWindowDelegate for ClipPanel {
        #[unsafe(method(windowShouldClose:))]
        fn should_close(&self, _: &NSWindow) -> bool {
            self.hide_panel();
            false
        }
        #[unsafe(method(windowDidResignKey:))]
        fn resigned(&self, _: &objc2_foundation::NSNotification) {
            self.hide_panel();
        }
        #[unsafe(method(windowDidResize:))]
        fn resized(&self, _: &objc2_foundation::NSNotification) {
            self.layout();
        }
    }

    unsafe impl NSTableViewDataSource for ClipPanel {
        #[unsafe(method(numberOfRowsInTableView:))]
        fn number_of_rows(&self, _: &NSTableView) -> NSInteger {
            self.ivars().rows.borrow().len() as NSInteger
        }
    }

    unsafe impl NSTableViewDelegate for ClipPanel {
        #[unsafe(method_id(tableView:viewForTableColumn:row:))]
        fn view_for(
            &self,
            _: &NSTableView,
            _: Option<&NSTableColumn>,
            row: NSInteger,
        ) -> Option<Retained<NSView>> {
            let hit = self
                .ivars()
                .rows
                .borrow()
                .get(row as usize)
                .cloned();
            hit.map(|hit| {
                let title = self.ivars().store.borrow().list_title(&hit);
                let thumbnail = if hit.kind == "image" || hit.kind == "file" {
                    let path = self.ivars().store.borrow().thumbnail_path(&hit.id);
                    path.and_then(|path| self.ivars().thumbnails.borrow_mut().get(&path))
                } else { None };
                design::cell(&title, &hit.kind, thumbnail.as_deref(), self.ivars().marked.borrow().contains(&hit.id), self.mtm())
            })
        }

        #[unsafe(method_id(tableView:rowViewForRow:))]
        fn row_view(&self, _: &NSTableView, _: NSInteger) -> Option<Retained<objc2_app_kit::NSTableRowView>> {
            Some(unsafe { Retained::cast_unchecked(design::ClipRow::new(self.mtm())) })
        }

        #[unsafe(method(tableViewSelectionDidChange:))]
        fn selection_changed(&self, _: &objc2_foundation::NSNotification) {
            self.show_selected();
        }
    }

    impl ClipPanel {
        #[unsafe(method(showSettings:))]
        fn settings_action(&self, _: &AnyObject) { self.show_settings(); }

        #[unsafe(method(showHistory:))]
        fn history_action(&self, _: &AnyObject) {
            self.set_filter(0);
            if !self.isVisible() { self.show_panel(); }
        }

        #[unsafe(method(showImages:))]
        fn images_action(&self, _: &AnyObject) {
            self.set_filter(3);
            if !self.isVisible() { self.show_panel(); }
        }
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key(&self) -> bool { true }

        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main(&self) -> bool { true }

        #[unsafe(method(filterChanged:))]
        fn filter_changed(&self, sender: &NSPopUpButton) {
            self.ivars().filter_index.set(sender.indexOfSelectedItem());
            self.refresh_results();
        }
        #[unsafe(method(zoomImageIn:))]
        fn zoom_in(&self, _: &AnyObject) { if let Some(zoom) = self.ivars().image_zoom.borrow().as_ref() { zoom.zoom_center(1.5); } }
        #[unsafe(method(zoomImageOut:))]
        fn zoom_out(&self, _: &AnyObject) { if let Some(zoom) = self.ivars().image_zoom.borrow().as_ref() { zoom.zoom_center(1.0 / 1.5); } }
        #[unsafe(method(fitImage:))]
        fn fit_image(&self, _: &AnyObject) { if let Some(zoom) = self.ivars().image_zoom.borrow().as_ref() { zoom.fit(); } }
        #[unsafe(method(tick:))]
        fn tick(&self, _: &NSTimer) {
            self.poll_hotkey();
            self.poll_pasteboard();
            self.poll_query();
            self.poll_ocr();
            self.poll_thumbnails();
            self.update_list_fade();
            if self.isVisible() {
                if let Some(video) = self.ivars().video.borrow().as_ref() { video.update(); }
            }
            if self.ivars().previewing.get() {
                if let (Some(zoom), Some(label)) = (self.ivars().image_zoom.borrow().as_ref(), self.ivars().zoom_status.borrow().as_ref()) {
                    label.setStringValue(&NSString::from_str(&format!("{:.0}%", zoom.magnification() * 100.0)));
                }
            }
        }

        #[unsafe(method(pasteNow:))]
        fn paste_now(&self, _: &NSTimer) { self.deliver_paste(); }
        #[unsafe(method(pasteNext:))]
        fn paste_next(&self, _: &NSTimer) { self.advance_paste(); }
    }
);

impl ClipPanel {
    fn mtm(&self) -> MainThreadMarker {
        MainThreadMarker::new().expect("main thread")
    }

    fn new(
        mtm: MainThreadMarker,
        store: Store,
        shortcuts: Rc<RefCell<crate::shortcuts::Service>>,
    ) -> Retained<Self> {
        let recent = store.recent(200);
        let rows = recent.clone();
        let frame = rect(0.0, 0.0, design::WIDTH, design::HEIGHT);
        let style = NSWindowStyleMask::Borderless;
        let this = mtm.alloc().set_ivars(Ivars {
            shortcuts,
            settings: RefCell::new(None),
            status_item: RefCell::new(None),
            store: RefCell::new(store),
            rows: RefCell::new(rows),
            recent: RefCell::new(recent),
            thumbnails: RefCell::new(crate::thumbnails::Thumbnails::new()),
            marked: RefCell::new(HashSet::new()),
            paste_queue: RefCell::new(VecDeque::new()),
            paste_timer: RefCell::new(None),
            pasting: Cell::new(false),
            query: RefCell::new(String::new()),
            search: RefCell::new(None),
            preview_title: RefCell::new(None),
            surface: RefCell::new(None),
            glass: RefCell::new(None),
            filter: RefCell::new(None),
            divider: RefCell::new(None),
            info: RefCell::new(Vec::new()),
            filter_index: Cell::new(0),
            table: RefCell::new(None),
            text: RefCell::new(None),
            image: RefCell::new(None),
            zoom_image: RefCell::new(None),
            image_zoom: RefCell::new(None),
            zoom_buttons: RefCell::new(Vec::new()),
            zoom_status: RefCell::new(None),
            document: RefCell::new(None),
            preview_kind: Cell::new(0),
            video: RefCell::new(None),
            meta: RefCell::new(None),
            list_scroll: RefCell::new(None),
            fade_state: Cell::new((-1, -1)),
            text_scroll: RefCell::new(None),
            capture: RefCell::new(crate::clipboard::Capture::new(0)),
            ignore_change: Cell::new(-1),
            prev_pid: Cell::new(0),
            previewing: Cell::new(false),
            ocr: crate::ocr::Worker::new(),
            ocr_pending: RefCell::new(HashSet::new()),
            ocr_scan_at: Cell::new(Instant::now()),
        });
        let this: Retained<Self> = unsafe {
            msg_send![
                super(this),
                initWithContentRect: frame,
                styleMask: style,
                backing: NSBackingStoreType::Buffered,
                defer: false,
            ]
        };
        this.setTitle(ns_string!("Clip"));
        this.setOpaque(false);
        this.setBackgroundColor(Some(&objc2_app_kit::NSColor::clearColor()));
        this.setHasShadow(true);
        unsafe {
            this.setReleasedWhenClosed(false);
        }
        this.setMovableByWindowBackground(true);
        for button in [
            objc2_app_kit::NSWindowButton::CloseButton,
            objc2_app_kit::NSWindowButton::MiniaturizeButton,
            objc2_app_kit::NSWindowButton::ZoomButton,
        ] {
            if let Some(view) = this.standardWindowButton(button) {
                view.setHidden(true);
            }
        }
        this.setLevel(objc2_app_kit::NSFloatingWindowLevel + 1);
        this.build_views(mtm);
        this.setDelegate(Some(ProtocolObject::from_ref(&*this)));
        let pb = NSPasteboard::generalPasteboard();
        *this.ivars().capture.borrow_mut() = crate::clipboard::Capture::new(pb.changeCount());
        this
    }

    fn build_views(&self, mtm: MainThreadMarker) {
        let glass = design::glass(
            rect(0.0, 0.0, design::WIDTH, design::HEIGHT),
            design::RADIUS,
            mtm,
        );
        // The window backing must follow the glass silhouette, not its square bounds.
        design::rounded(
            &glass,
            design::RADIUS,
            &objc2_app_kit::NSColor::clearColor(),
        );
        let content: Retained<NSView> =
            unsafe { Retained::cast_unchecked(design::PanelSurface::new(glass.bounds(), mtm)) };
        glass.setContentView(Some(&content));
        self.setContentView(Some(&glass));
        *self.ivars().glass.borrow_mut() = Some(glass);
        *self.ivars().surface.borrow_mut() = Some(content.clone());
        let search =
            NSTextField::initWithFrame(NSTextField::alloc(mtm), rect(0.0, 0.0, 10.0, 10.0));
        search.setPlaceholderString(Some(ns_string!("Type to filter entries…")));
        search.setFont(Some(&objc2_app_kit::NSFont::systemFontOfSize(19.0)));
        search.setBezeled(false);
        search.setBordered(false);
        search.setDrawsBackground(false);
        search.setFocusRingType(objc2_app_kit::NSFocusRingType::None);
        content.addSubview(&search);
        let preview_title = design::label("Preview", 19.0, mtm);
        preview_title.setHidden(true);
        content.addSubview(&preview_title);
        *self.ivars().preview_title.borrow_mut() = Some(preview_title);
        let filter = NSPopUpButton::initWithFrame_pullsDown(
            NSPopUpButton::alloc(mtm),
            rect(0.0, 0.0, 110.0, 28.0),
            false,
        );
        filter.setBordered(false);
        filter.setFont(Some(&objc2_app_kit::NSFont::systemFontOfSize(14.0)));
        for title in ["All Types", "Text", "Links", "Images", "Files"] {
            filter.addItemWithTitle(&NSString::from_str(title));
        }
        unsafe {
            filter.setTarget(Some(self));
            filter.setAction(Some(objc2::sel!(filterChanged:)));
        }
        content.addSubview(&filter);
        let divider = NSView::initWithFrame(NSView::alloc(mtm), rect(0.0, 0.0, 0.5, 10.0));
        design::rounded(
            &divider,
            0.0,
            &objc2_app_kit::NSColor::labelColor().colorWithAlphaComponent(0.055),
        );
        content.addSubview(&divider);

        let table = NSTableView::initWithFrame(
            NSTableView::alloc(mtm),
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(10.0, 10.0)),
        );
        let column =
            NSTableColumn::initWithIdentifier(NSTableColumn::alloc(mtm), ns_string!("main"));
        table.addTableColumn(&column);
        table.setHeaderView(None);
        table.setRowHeight(design::ROW);
        table.setIntercellSpacing(NSSize::new(0.0, 0.0));
        table.setBackgroundColor(&objc2_app_kit::NSColor::clearColor());
        table.setStyle(objc2_app_kit::NSTableViewStyle::Plain);
        table.setFocusRingType(objc2_app_kit::NSFocusRingType::None);
        unsafe {
            let src = ProtocolObject::from_ref(self);
            table.setDataSource(Some(src));
            table.setDelegate(Some(ProtocolObject::from_ref(self)));
        }
        let list_scroll = design::scroll(&table, mtm);
        // Keep selection marks unobstructed. Wheel/trackpad and keyboard
        // scrolling work without a visible scroller.
        list_scroll.setHasVerticalScroller(false);
        content.addSubview(&list_scroll);

        let text = NSTextView::initWithFrame(
            NSTextView::alloc(mtm),
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(10.0, 10.0)),
        );
        text.setEditable(false);
        text.setSelectable(true);
        text.setRichText(false);
        text.setDrawsBackground(false);
        text.setTextColor(Some(&objc2_app_kit::NSColor::labelColor()));
        text.setTextContainerInset(NSSize::new(6.0, 6.0));
        text.setFont(Some(
            &objc2_app_kit::NSFont::monospacedSystemFontOfSize_weight(14.0, unsafe {
                objc2_app_kit::NSFontWeightRegular
            }),
        ));
        let text_scroll = design::scroll(&text, mtm);
        content.addSubview(&text_scroll);

        let image = NSImageView::initWithFrame(
            NSImageView::alloc(mtm),
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(10.0, 10.0)),
        );
        image.setImageScaling(objc2_app_kit::NSImageScaling::ScaleProportionallyUpOrDown);
        content.addSubview(&image);
        let zoom_image =
            NSImageView::initWithFrame(NSImageView::alloc(mtm), rect(0.0, 0.0, 100.0, 100.0));
        zoom_image.setImageScaling(objc2_app_kit::NSImageScaling::ScaleProportionallyUpOrDown);
        let image_zoom = crate::image_zoom::ImageZoom::new(mtm);
        image_zoom.setDocumentView(Some(&zoom_image));
        image_zoom.setHidden(true);
        content.addSubview(&image_zoom);
        let mut zoom_buttons = Vec::new();
        for (title, action) in [
            ("−", objc2::sel!(zoomImageOut:)),
            ("Fit", objc2::sel!(fitImage:)),
            ("+", objc2::sel!(zoomImageIn:)),
        ] {
            let button = NSButton::initWithFrame(NSButton::alloc(mtm), rect(0.0, 0.0, 40.0, 30.0));
            button.setTitle(&NSString::from_str(title));
            button.setBordered(false);
            button.setFont(Some(&objc2_app_kit::NSFont::systemFontOfSize(15.0)));
            unsafe {
                button.setTarget(Some(self));
                button.setAction(Some(action));
            }
            button.setHidden(true);
            content.addSubview(&button);
            zoom_buttons.push(button);
        }
        let zoom_status = design::label("100%", 12.0, mtm);
        zoom_status.setAlignment(objc2_app_kit::NSTextAlignment::Center);
        zoom_status.setHidden(true);
        content.addSubview(&zoom_status);
        *self.ivars().zoom_buttons.borrow_mut() = zoom_buttons;
        *self.ivars().zoom_status.borrow_mut() = Some(zoom_status);
        *self.ivars().image_zoom.borrow_mut() = Some(image_zoom);
        *self.ivars().zoom_image.borrow_mut() = Some(zoom_image);

        let document = unsafe {
            let view = QLPreviewView::initWithFrame_style(
                QLPreviewView::alloc(mtm),
                NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(10.0, 10.0)),
                QLPreviewViewStyle::Compact,
            );
            if let Some(view) = &view {
                view.setAutostarts(false);
                view.setHidden(true);
                content.addSubview(view);
            }
            view
        };

        let meta = design::label("Information", 12.0, mtm);
        meta.setTextColor(Some(&objc2_app_kit::NSColor::secondaryLabelColor()));
        content.addSubview(&meta);

        let mut info = Vec::new();
        for index in 0..5 {
            let row = NSView::initWithFrame(NSView::alloc(mtm), rect(0.0, 0.0, 10.0, 28.0));
            if index % 2 == 0 {
                design::rounded(
                    &row,
                    8.0,
                    &objc2_app_kit::NSColor::labelColor().colorWithAlphaComponent(0.035),
                );
            }
            let key = design::label("", 12.0, mtm);
            key.setTextColor(Some(&objc2_app_kit::NSColor::secondaryLabelColor()));
            let value = design::label("", 12.0, mtm);
            value.setAlignment(objc2_app_kit::NSTextAlignment::Right);
            row.addSubview(&key);
            row.addSubview(&value);
            content.addSubview(&row);
            info.push((row, key, value));
        }
        *self.ivars().info.borrow_mut() = info;
        *self.ivars().filter.borrow_mut() = Some(filter);
        *self.ivars().divider.borrow_mut() = Some(divider);

        *self.ivars().search.borrow_mut() = Some(search);
        *self.ivars().table.borrow_mut() = Some(table);
        *self.ivars().text.borrow_mut() = Some(text);
        *self.ivars().image.borrow_mut() = Some(image);
        *self.ivars().document.borrow_mut() = document;
        let video = crate::video::VideoPreview::new(mtm);
        content.addSubview(&video);
        *self.ivars().video.borrow_mut() = Some(video);
        *self.ivars().meta.borrow_mut() = Some(meta);
        *self.ivars().list_scroll.borrow_mut() = Some(list_scroll);
        *self.ivars().text_scroll.borrow_mut() = Some(text_scroll);
        self.layout();
        self.select_row(0);
    }

    fn layout(&self) {
        let bounds = self.contentView().expect("content").bounds();
        let w = bounds.size.width;
        let h = bounds.size.height;
        let expanded = self.ivars().previewing.get();
        let zooming_image = expanded && self.ivars().preview_kind.get() == 1;
        if let Some(zoom) = self.ivars().image_zoom.borrow().as_ref() {
            zoom.setHidden(!zooming_image);
        }
        if let Some(image) = self.ivars().image.borrow().as_ref() {
            image.setHidden(self.ivars().preview_kind.get() != 1 || zooming_image);
        }
        for (i, button) in self.ivars().zoom_buttons.borrow().iter().enumerate() {
            button.setHidden(!zooming_image);
            button.setFrame(rect(w / 2.0 - 92.0 + i as f64 * 42.0, 11.0, 40.0, 30.0));
        }
        if let Some(label) = self.ivars().zoom_status.borrow().as_ref() {
            label.setHidden(!zooming_image);
            label.setFrame(rect(w / 2.0 + 38.0, 17.0, 54.0, 18.0));
        }
        let has_selection = !self.ivars().rows.borrow().is_empty();
        if let Some(search) = self.ivars().search.borrow().as_ref() {
            search.setHidden(expanded);
        }
        if let Some(title) = self.ivars().preview_title.borrow().as_ref() {
            title.setHidden(!expanded);
            title.setFrame(rect(16.0, h - 44.0, w - 32.0, 28.0));
        }
        if let Some(filter) = self.ivars().filter.borrow().as_ref() {
            filter.setHidden(expanded);
        }
        if let Some(list) = self.ivars().list_scroll.borrow().as_ref() {
            list.setHidden(expanded);
        }
        if let Some(divider) = self.ivars().divider.borrow().as_ref() {
            divider.setHidden(expanded);
        }
        if let Some(meta) = self.ivars().meta.borrow().as_ref() {
            meta.setHidden(expanded || !has_selection);
        }
        for (row, _, _) in self.ivars().info.borrow().iter() {
            row.setHidden(expanded || !has_selection);
        }
        if let Some(text) = self.ivars().text_scroll.borrow().as_ref() {
            text.setHidden(
                matches!(self.ivars().preview_kind.get(), 2 | 3)
                    || (expanded && self.ivars().preview_kind.get() == 1),
            );
        }
        let pad = 16.0;
        let meta_h = 164.0;
        let list_w = design::LIST_WIDTH;
        if let Some(filter) = self.ivars().filter.borrow().as_ref() {
            filter.setFrame(rect(w - 130.0, h - 43.0, 114.0, 28.0));
        }
        if let Some(divider) = self.ivars().divider.borrow().as_ref() {
            divider.setFrame(rect(
                list_w,
                design::FOOTER,
                0.5,
                h - design::HEADER - design::FOOTER,
            ));
        }
        if let Some(search) = self.ivars().search.borrow().as_ref() {
            search.setFrame(rect(16.0, h - 44.0, w - 158.0, 28.0));
        }
        let body_top = h - design::HEADER - 8.0;
        let body_h = body_top - design::FOOTER;
        if let Some(list) = self.ivars().list_scroll.borrow().as_ref() {
            list.setFrame(NSRect::new(
                NSPoint::new(8.0, design::FOOTER),
                NSSize::new(list_w - 16.0, body_h),
            ));
            self.ivars().fade_state.set((-1, -1));
        }
        let preview_x = list_w + pad;
        let preview_w = (w - preview_x - pad).max(40.0);
        if let Some(meta) = self.ivars().meta.borrow().as_ref() {
            meta.setFrame(NSRect::new(
                NSPoint::new(preview_x, design::FOOTER + 146.0),
                NSSize::new(preview_w, 18.0),
            ));
        }
        for (i, (row, key, value)) in self.ivars().info.borrow().iter().enumerate() {
            row.setFrame(rect(
                preview_x,
                design::FOOTER + (4 - i) as f64 * 28.0,
                preview_w,
                28.0,
            ));
            key.setFrame(rect(10.0, 6.0, 105.0, 17.0));
            value.setFrame(rect(120.0, 6.0, preview_w - 130.0, 17.0));
        }
        let preview_h = (h - design::HEADER - design::FOOTER - meta_h - 14.0).max(40.0);
        let image_fraction = if self.ivars().preview_kind.get() == 1 {
            0.75
        } else {
            0.0
        };
        if let Some(image) = self.ivars().image.borrow().as_ref() {
            image.setFrame(NSRect::new(
                NSPoint::new(
                    preview_x,
                    design::FOOTER + meta_h + 6.0 + preview_h * (1.0 - image_fraction),
                ),
                NSSize::new(preview_w, preview_h * image_fraction),
            ));
        }
        if let Some(text) = self.ivars().text_scroll.borrow().as_ref() {
            text.setFrame(NSRect::new(
                NSPoint::new(preview_x, design::FOOTER + meta_h + 6.0),
                NSSize::new(preview_w, preview_h * (1.0 - image_fraction)),
            ));
        }
        if let Some(document) = self.ivars().document.borrow().as_ref() {
            document.setFrame(NSRect::new(
                NSPoint::new(preview_x, design::FOOTER + meta_h + 6.0),
                NSSize::new(preview_w, preview_h),
            ));
        }
        if let Some(video) = self.ivars().video.borrow().as_ref() {
            video.setHidden(self.ivars().preview_kind.get() != 3);
            let frame = if expanded { design::expanded_preview_frame(w, h) } else {
                rect(preview_x, design::FOOTER + meta_h + 6.0, preview_w, preview_h)
            };
            video.layout_video(frame, expanded);
        }
        if expanded {
            let frame = design::expanded_preview_frame(w, h);
            if let Some(image) = self.ivars().image.borrow().as_ref() {
                image.setFrame(frame);
            }
            if let Some(zoom) = self.ivars().image_zoom.borrow().as_ref() {
                zoom.setFrame(frame);
                if let Some(image) = self.ivars().zoom_image.borrow().as_ref() {
                    zoom.resize_document(image);
                }
            }
            if let Some(document) = self.ivars().document.borrow().as_ref() {
                document.setFrame(frame);
            }
            if let Some(text) = self.ivars().text_scroll.borrow().as_ref() {
                text.setFrame(frame);
            }
        }
    }

    fn show_panel(&self) {
        self.cancel_paste();
        self.ivars().previewing.set(false);
        if let Some(app) = NSWorkspace::sharedWorkspace()
            .frontmostApplication()
            .filter(|app| {
                app.processIdentifier()
                    != NSRunningApplication::currentApplication().processIdentifier()
            })
        {
            self.ivars().prev_pid.set(app.processIdentifier());
        }
        if let Some(screen) = NSScreen::mainScreen(self.mtm()) {
            let vis = screen.visibleFrame();
            let size = NSSize::new(design::WIDTH, design::HEIGHT);
            self.setFrame_display(
                NSRect::new(
                    NSPoint::new(
                        vis.origin.x + (vis.size.width - size.width) / 2.0,
                        vis.origin.y + (vis.size.height - size.height) / 2.0,
                    ),
                    size,
                ),
                true,
            );
        }
        self.layout();
        self.makeKeyAndOrderFront(None);
        if self.ivars().preview_kind.get() == 3 {
            if let Some(video) = self.ivars().video.borrow().as_ref() { video.play(); }
        }
        self.refresh_thumbnail_rows();
        NSApplication::sharedApplication(self.mtm()).activateIgnoringOtherApps(true);
        if let Some(table) = self.ivars().table.borrow().as_ref() {
            self.makeFirstResponder(Some(table));
        }
    }

    fn resize_for_preview(&self, expanded: bool) {
        if let Some(screen) = self.screen().or_else(|| NSScreen::mainScreen(self.mtm())) {
            let visible = screen.visibleFrame();
            let size = if expanded {
                design::preview_window_size(visible.size)
            } else {
                NSSize::new(design::WIDTH, design::HEIGHT)
            };
            self.setFrame_display(
                rect(
                    visible.origin.x + (visible.size.width - size.width) / 2.0,
                    visible.origin.y + (visible.size.height - size.height) / 2.0,
                    size.width,
                    size.height,
                ),
                true,
            );
        }
    }

    fn hide_panel(&self) {
        self.orderOut(None);
        if let Some(video) = self.ivars().video.borrow().as_ref() { video.pause(); }
        // End field editing before resetting, so its editor cannot restore a
        // stale query on the next open. Preview navigation never comes here.
        self.makeFirstResponder(None);
        if let Some(search) = self.ivars().search.borrow().as_ref() {
            search.setStringValue(ns_string!(""));
        }
        self.ivars().query.borrow_mut().clear();
        self.refresh_results();
    }

    fn poll_hotkey(&self) {
        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            let (history, images, active) = {
                let shortcuts = self.ivars().shortcuts.borrow();
                (
                    shortcuts.history_id(),
                    shortcuts.images_id(),
                    shortcuts.accepts(event.id),
                )
            };
            if !active {
                continue;
            }
            if event.state == HotKeyState::Pressed {
                if event.id == images {
                    let index = if self.isVisible() && self.ivars().filter_index.get() == 3 {
                        0
                    } else {
                        3
                    };
                    self.set_filter(index);
                    if !self.isVisible() {
                        self.show_panel();
                    } else if self.ivars().previewing.get() {
                        self.focus_search();
                    }
                    continue;
                }
                if event.id != history {
                    continue;
                }
                if self.isVisible() {
                    self.hide_panel();
                } else {
                    self.set_filter(0);
                    self.show_panel();
                }
            }
        }
    }

    fn set_filter(&self, index: isize) {
        self.ivars().filter_index.set(index);
        if let Some(filter) = self.ivars().filter.borrow().as_ref() {
            filter.selectItemAtIndex(index);
        }
        self.refresh_results();
    }

    fn show_settings(&self) {
        if let Some(app) = NSWorkspace::sharedWorkspace()
            .frontmostApplication()
            .filter(|app| {
                app.processIdentifier()
                    != NSRunningApplication::currentApplication().processIdentifier()
            })
        {
            self.ivars().prev_pid.set(app.processIdentifier());
        }
        self.cancel_paste();
        if self.isVisible() {
            self.hide_panel();
        }
        let existing = self.ivars().settings.borrow().clone();
        let settings = existing.unwrap_or_else(|| {
            let window = crate::settings::SettingsWindow::new(
                self.mtm(),
                self.ivars().shortcuts.clone(),
                self.ivars()
                    .status_item
                    .borrow()
                    .as_ref()
                    .expect("menu-bar item initialized")
                    .clone(),
            );
            *self.ivars().settings.borrow_mut() = Some(window.clone());
            window
        });
        settings.show();
    }

    fn poll_query(&self) {
        let Some(search) = self.ivars().search.borrow().clone() else {
            return;
        };
        let text = search.stringValue().to_string();
        if text == *self.ivars().query.borrow() {
            return;
        }
        *self.ivars().query.borrow_mut() = text.clone();
        self.refresh_results();
    }

    fn refresh_results(&self) {
        let text = self.ivars().query.borrow().clone();
        let kind = match self.ivars().filter_index.get() {
            1 => Some("text"),
            2 => Some("link"),
            3 => Some("image"),
            4 => Some("file"),
            _ => None,
        };
        let hits = if kind.is_some() {
            self.ivars().store.borrow().filtered(&text, 200, kind)
        } else if text.trim().is_empty() {
            self.ivars().recent.borrow().clone()
        } else {
            self.ivars().store.borrow().search(&text, 80)
        };
        *self.ivars().rows.borrow_mut() = hits;
        if let Some(table) = self.ivars().table.borrow().as_ref() {
            table.reloadData();
        }
        self.select_row(0);
    }

    fn poll_ocr(&self) {
        let mut updated = false;
        while let Ok(result) = self.ivars().ocr.results.try_recv() {
            self.ivars().ocr_pending.borrow_mut().remove(&result.id);
            if let Err(error) = self.ivars().store.borrow_mut().finish_ocr(&result) {
                eprintln!("clip: could not save OCR result: {error}");
            } else {
                updated = true;
            }
        }
        if updated {
            self.show_selected();
            if !self.ivars().query.borrow().is_empty() {
                self.ivars().query.borrow_mut().clear();
                self.poll_query();
            }
        }
        let now = Instant::now();
        if now < self.ivars().ocr_scan_at.get() {
            return;
        }
        self.ivars().ocr_scan_at.set(now + Duration::from_secs(1));
        let jobs = match self.ivars().store.borrow().pending_ocr() {
            Ok(jobs) => jobs,
            Err(error) => {
                eprintln!("clip: could not read OCR queue: {error}");
                return;
            }
        };
        for job in jobs {
            if self.ivars().ocr_pending.borrow().contains(&job.id) {
                continue;
            }
            let id = job.id.clone();
            if self.ivars().ocr.submit(job).is_ok() {
                self.ivars().ocr_pending.borrow_mut().insert(id);
            }
        }
    }

    fn poll_thumbnails(&self) {
        let changed = self.ivars().thumbnails.borrow_mut().poll();
        if changed && self.isVisible() && !self.ivars().previewing.get() {
            self.refresh_thumbnail_rows();
        }
    }

    fn refresh_thumbnail_rows(&self) {
        let Some(table) = self.ivars().table.borrow().clone() else {
            return;
        };
        let selected = table.selectedRow();
        let rows = table.rowsInRect(table.visibleRect());
        if rows.length > 0 {
            table.reloadDataForRowIndexes_columnIndexes(
                &NSIndexSet::indexSetWithIndexesInRange(rows),
                &NSIndexSet::indexSetWithIndex(0),
            );
            if selected >= 0 {
                self.select_row(selected);
            }
        }
    }

    fn select_row(&self, row: isize) {
        let Some(table) = self.ivars().table.borrow().clone() else {
            return;
        };
        let n = self.ivars().rows.borrow().len() as isize;
        if n == 0 || row < 0 || row >= n {
            self.show_selected();
            return;
        }
        let indexes = NSIndexSet::indexSetWithIndex(row as usize);
        table.selectRowIndexes_byExtendingSelection(&indexes, false);
        table.scrollRowToVisible(row);
        self.show_selected();
    }

    fn show_selected(&self) {
        let row = self
            .ivars()
            .table
            .borrow()
            .as_ref()
            .map(|t| t.selectedRow())
            .unwrap_or(-1);
        let id = self
            .ivars()
            .rows
            .borrow()
            .get(row as usize)
            .map(|h| h.id.clone());
        let Some(id) = id else {
            if let Some(video) = self.ivars().video.borrow().as_ref() { video.load(None, false); }
            if let Some(text) = self.ivars().text.borrow().as_ref() {
                text.setString(ns_string!("No matching entries"));
            }
            if let Some(image) = self.ivars().image.borrow().as_ref() {
                image.setImage(None);
                image.setHidden(true);
            }
            if let Some(document) = self.ivars().document.borrow().as_ref() {
                document.setHidden(true);
            }
            if let Some(scroll) = self.ivars().text_scroll.borrow().as_ref() {
                scroll.setHidden(false);
            }
            for (row, key, value) in self.ivars().info.borrow().iter() {
                row.setHidden(true);
                key.setStringValue(ns_string!(""));
                value.setStringValue(ns_string!(""));
            }
            if let Some(meta) = self.ivars().meta.borrow().as_ref() {
                meta.setHidden(true);
            }
            self.ivars().preview_kind.set(0);
            self.layout();
            return;
        };
        let Some(preview) = self.ivars().store.borrow().preview(&id) else {
            return;
        };
        if let Some(meta) = self.ivars().meta.borrow().as_ref() {
            meta.setHidden(false);
        }
        for (row, _, _) in self.ivars().info.borrow().iter() {
            row.setHidden(false);
        }
        let is_video = preview.file_path.as_ref().is_some_and(|p| p.exists() && crate::video::is_video(p));
        if let Some(video) = self.ivars().video.borrow().as_ref() {
            video.load(preview.file_path.as_deref().filter(|_| is_video), self.isVisible());
        }
        let is_document = !is_video && preview.image_path.is_none()
            && preview.file_path.as_ref().is_some_and(|p| p.exists())
            && self.ivars().document.borrow().is_some();
        self.ivars().preview_kind.set(if is_video {
            3
        } else if is_document {
            2
        } else if preview.image_path.is_some() {
            1
        } else {
            0
        });
        if let Some(document) = self.ivars().document.borrow().as_ref() {
            if let Some(path) = preview.file_path.as_ref().filter(|_| is_document) {
                let url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
                unsafe {
                    document.setPreviewItem(Some(ProtocolObject::from_ref(&*url)));
                }
            }
            document.setHidden(!is_document);
        }
        if let Some(scroll) = self.ivars().text_scroll.borrow().as_ref() {
            scroll.setHidden(is_document || is_video);
        }
        self.layout();
        if let Some(text) = self.ivars().text.borrow().as_ref() {
            text.setString(&NSString::from_str(&preview.text));
        }
        if let Some(image) = self.ivars().image.borrow().as_ref() {
            let path = preview.image_path.clone().or(preview.file_path.clone());
            if let Some(path) = path.filter(|_| !is_document && !is_video) {
                if preview.image_path.is_some() {
                    image.setImage(
                        NSImage::initWithContentsOfFile(
                            NSImage::alloc(),
                            &NSString::from_str(&path.display().to_string()),
                        )
                        .as_deref(),
                    );
                } else {
                    image.setImage(Some(
                        &NSWorkspace::sharedWorkspace()
                            .iconForFile(&NSString::from_str(&path.display().to_string())),
                    ));
                }
                image.setHidden(self.ivars().previewing.get());
            } else {
                image.setImage(None);
                image.setHidden(true);
            }
        }
        let values = [
            ("Source", preview.source.clone()),
            ("Type", preview.kind.clone()),
            ("Copied", store::format_when(&preview.created_at)),
            ("Characters", preview.text.chars().count().to_string()),
            (
                "Size",
                preview
                    .bytes
                    .map(|n| format!("{n} bytes"))
                    .unwrap_or_else(|| format!("{} bytes", preview.text.len())),
            ),
        ];
        for ((_, key, value), (name, data)) in self.ivars().info.borrow().iter().zip(values) {
            key.setStringValue(&NSString::from_str(name));
            value.setStringValue(&NSString::from_str(&data));
        }
    }

    fn move_selection(&self, delta: isize) {
        let current = self
            .ivars()
            .table
            .borrow()
            .as_ref()
            .map(|t| t.selectedRow())
            .unwrap_or(0);
        let next = (current + delta).max(0);
        self.select_row(next);
    }

    fn focus_preview(&self) {
        if self.ivars().previewing.get() || self.ivars().rows.borrow().is_empty() {
            return;
        }
        if let Some(table) = self.ivars().table.borrow().as_ref() {
            if let Some(hit) = self.ivars().rows.borrow().get(table.selectedRow() as usize) {
                if let Some(title) = self.ivars().preview_title.borrow().as_ref() {
                    title.setStringValue(&NSString::from_str(&hit.title.replace('\n', " ")));
                }
            }
        }
        self.ivars().previewing.set(true);
        if let (Some(zoom), Some(image), Some(large_image)) = (
            self.ivars().image_zoom.borrow().as_ref(),
            self.ivars().image.borrow().as_ref(),
            self.ivars().zoom_image.borrow().as_ref(),
        ) {
            large_image.setImage(image.image().as_deref());
            zoom.fit();
        }
        self.resize_for_preview(true);
        self.layout();
        if matches!(self.ivars().preview_kind.get(), 1 | 3) {
            self.makeFirstResponder(None);
            return;
        }
        if self.ivars().preview_kind.get() == 2 {
            if let Some(document) = self.ivars().document.borrow().as_ref() {
                self.makeFirstResponder(Some(document));
            }
            return;
        }
        if let Some(text) = self.ivars().text.borrow().as_ref() {
            self.makeFirstResponder(Some(text));
        }
    }

    fn focus_search(&self) {
        self.ivars().previewing.set(false);
        self.resize_for_preview(false);
        self.layout();
        if let Some(search) = self.ivars().search.borrow().as_ref() {
            self.makeFirstResponder(Some(search));
        }
    }

    fn list_is_focused(&self) -> bool {
        self.ivars().table.borrow().as_ref().is_some_and(|table| {
            self.firstResponder().is_some_and(|responder| {
                Retained::as_ptr(&responder).cast::<AnyObject>()
                    == Retained::as_ptr(table).cast::<AnyObject>()
            })
        })
    }

    fn toggle_mark(&self) {
        let row = self
            .ivars()
            .table
            .borrow()
            .as_ref()
            .map(|t| t.selectedRow())
            .unwrap_or(-1);
        let id = self
            .ivars()
            .rows
            .borrow()
            .get(row as usize)
            .map(|hit| hit.id.clone());
        if let Some(id) = id {
            {
                let mut marked = self.ivars().marked.borrow_mut();
                if !marked.remove(&id) {
                    marked.insert(id);
                }
            }
            self.reload_marks();
        }
    }

    fn reload_marks(&self) {
        if let Some(table) = self.ivars().table.borrow().as_ref() {
            let row = table.selectedRow();
            table.reloadData();
            if row >= 0 {
                table.selectRowIndexes_byExtendingSelection(
                    &NSIndexSet::indexSetWithIndex(row as usize),
                    false,
                );
            }
        }
    }

    // The list dissolves at an edge only while more rows lie beyond it.
    fn update_list_fade(&self) {
        const FADE: f64 = 36.0;
        let list = self.ivars().list_scroll.borrow();
        let Some(list) = list.as_ref() else {
            return;
        };
        if list.isHidden() {
            return;
        }
        let clip = list.contentView();
        let visible = clip.bounds();
        let Some(document) = list.documentView() else {
            return;
        };
        let content_h = document.frame().size.height;
        let top = visible.origin.y.clamp(0.0, FADE);
        let bottom = (content_h - visible.origin.y - visible.size.height).clamp(0.0, FADE);
        let state = (top.round() as i32, bottom.round() as i32);
        if state == self.ivars().fade_state.get() {
            return;
        }
        self.ivars().fade_state.set(state);
        design::edge_fade(list, list.frame().size, top, bottom);
    }

    fn paste_selected(&self) {
        if self.ivars().pasting.get() {
            return;
        }
        let row = self
            .ivars()
            .table
            .borrow()
            .as_ref()
            .map(|t| t.selectedRow())
            .unwrap_or(-1);
        let id = self
            .ivars()
            .rows
            .borrow()
            .get(row as usize)
            .map(|h| h.id.clone());
        let mut ids = self.ivars().marked.borrow().clone();
        if ids.is_empty() {
            if let Some(id) = id {
                ids.insert(id);
            }
        }
        let previews = match self.ivars().store.borrow().selected_previews(&ids) {
            Ok(previews) => previews,
            Err(error) => {
                eprintln!("clip: cannot paste selection: {error}");
                return;
            }
        };
        let mut plan = crate::selection::paste_plan(previews);
        let Some(first) = plan.pop_front() else {
            return;
        };
        if !self.write_preview(&first) {
            return;
        }
        *self.ivars().paste_queue.borrow_mut() = plan;
        self.ivars().pasting.set(true);
        self.ivars().marked.borrow_mut().clear();
        self.reload_marks();
        let pid = self.ivars().prev_pid.get();
        self.hide_panel();
        if let Some(app) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid) {
            app.activateWithOptions(NSApplicationActivationOptions::ActivateIgnoringOtherApps);
        }
        self.schedule_paste(0.08, objc2::sel!(pasteNow:));
    }

    fn write_preview(&self, preview: &store::Preview) -> bool {
        let pb = NSPasteboard::generalPasteboard();
        if let Some(path) = preview.image_path.as_ref() {
            if let Ok(bytes) = std::fs::read(path) {
                let data = objc2_foundation::NSData::from_vec(bytes);
                let data_type = if path
                    .extension()
                    .is_some_and(|ext| ext == "tiff" || ext == "tif")
                {
                    unsafe { NSPasteboardTypeTIFF }
                } else {
                    unsafe { NSPasteboardTypePNG }
                };
                pb.clearContents();
                if !pb.setData_forType(Some(&data), data_type) {
                    return false;
                }
                if let Some(file) = preview.file_path.as_ref().filter(|p| p.exists()) {
                    let url = NSURL::fileURLWithPath(&NSString::from_str(&file.to_string_lossy()));
                    if let Some(url_string) = url.absoluteString() {
                        pb.setString_forType(&url_string, unsafe { NSPasteboardTypeFileURL });
                    }
                }
            } else {
                eprintln!("clip: selected image is unavailable");
                return false;
            }
        } else if let Some(path) = preview.file_path.as_ref() {
            if !path.exists() {
                eprintln!("clip: selected file is unavailable");
                return false;
            }
            let url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
            let Some(url_string) = url.absoluteString() else {
                return false;
            };
            pb.clearContents();
            if !pb.setString_forType(&url_string, unsafe { NSPasteboardTypeFileURL }) {
                return false;
            }
        } else {
            pb.clearContents();
            if !pb.setString_forType(&NSString::from_str(&preview.text), unsafe {
                NSPasteboardTypeString
            }) {
                return false;
            }
        }
        self.ivars().ignore_change.set(pb.changeCount());
        true
    }

    fn schedule_paste(&self, delay: f64, selector: objc2::runtime::Sel) {
        if let Some(timer) = self.ivars().paste_timer.borrow_mut().take() {
            timer.invalidate();
        }
        let timer = unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                delay, self, selector, None, false,
            )
        };
        *self.ivars().paste_timer.borrow_mut() = Some(timer);
    }

    fn cancel_paste(&self) {
        self.ivars().pasting.set(false);
        self.ivars().paste_queue.borrow_mut().clear();
        if let Some(timer) = self.ivars().paste_timer.borrow_mut().take() {
            timer.invalidate();
        }
    }

    fn paste_target_unchanged(&self) -> bool {
        self.ivars().pasting.get()
            && NSWorkspace::sharedWorkspace()
                .frontmostApplication()
                .is_some_and(|app| app.processIdentifier() == self.ivars().prev_pid.get())
            && NSPasteboard::generalPasteboard().changeCount() == self.ivars().ignore_change.get()
    }

    fn deliver_paste(&self) {
        if !self.paste_target_unchanged() {
            self.cancel_paste();
            return;
        }
        if !paste::accessibility_trusted() {
            paste::post_paste();
            self.cancel_paste();
            return;
        }
        paste::post_paste();
        if self.ivars().paste_queue.borrow().is_empty() {
            self.cancel_paste();
        } else {
            // Mixed/media selections require distinct native paste operations.
            self.schedule_paste(0.25, objc2::sel!(pasteNext:));
        }
    }

    fn advance_paste(&self) {
        if !self.paste_target_unchanged() {
            self.cancel_paste();
            return;
        }
        let next = self.ivars().paste_queue.borrow_mut().pop_front();
        if let Some(preview) = next {
            if self.write_preview(&preview) {
                self.schedule_paste(0.05, objc2::sel!(pasteNow:));
            } else {
                self.cancel_paste();
            }
        } else {
            self.cancel_paste();
        }
    }

    fn poll_pasteboard(&self) {
        let pb = NSPasteboard::generalPasteboard();
        let count = pb.changeCount();
        if count == self.ivars().ignore_change.get() {
            return;
        }
        let (source, bundle) = front_app();
        if matches!(
            bundle.as_str(),
            "com.apple.keychainaccess" | "com.apple.Passwords" | "com.apple.passwordmanager"
        ) {
            return;
        }
        let source = if source.is_empty() {
            None
        } else {
            Some(source)
        };
        let mut capture = self.ivars().capture.borrow_mut();
        if !capture.begin(count, source) {
            return;
        }
        let payload = capture.read(&pb);
        // A provider can replace the clipboard while rendering promised data.
        if pb.changeCount() != count {
            return;
        }
        let source = capture.source.clone();
        drop(capture);
        let mut captured = false;
        if let Some(text) = payload.text {
            self.ivars()
                .store
                .borrow_mut()
                .insert_text(&text, source.as_deref());
            captured = true;
        }
        let file_path = payload
            .file_url
            .map(|file| percent_decode(file.trim_start_matches("file://")));
        let file_path = file_path.as_deref().map(std::path::Path::new);
        if let Some((bytes, ext)) = payload.image {
            self.ivars().store.borrow_mut().insert_image_with_file(
                &bytes,
                ext,
                file_path,
                source.as_deref(),
            );
            captured = true;
        } else if let Some(path) = file_path {
            self.ivars()
                .store
                .borrow_mut()
                .insert_file(path, source.as_deref());
            captured = true;
        }
        if !captured {
            return;
        }
        // A duplicate also changes its recency.
        {
            let recent = self.ivars().store.borrow().recent(200);
            *self.ivars().recent.borrow_mut() = recent;
            self.refresh_results();
        }
    }

    fn handle_key(&self, event: &NSEvent) -> bool {
        if !self.isVisible() {
            return false;
        }
        let plain = !event.modifierFlags().intersects(
            objc2_app_kit::NSEventModifierFlags::Command
                | objc2_app_kit::NSEventModifierFlags::Control
                | objc2_app_kit::NSEventModifierFlags::Option,
        );
        if plain && !self.ivars().previewing.get() && self.list_is_focused() {
            if let Some(chars) = event.charactersIgnoringModifiers() {
                let chars = chars.to_string();
                if chars.eq_ignore_ascii_case("x") {
                    if !event.isARepeat() {
                        self.toggle_mark();
                    }
                    return true;
                }
                if chars
                    .chars()
                    .any(|c| !c.is_control() && !(0xf700..=0xf8ff).contains(&(c as u32)))
                {
                    self.focus_search();
                    return false;
                }
            }
        }
        if plain && event.keyCode() == 49 && self.ivars().previewing.get() && self.ivars().preview_kind.get() == 3 {
            if !event.isARepeat() {
                if let Some(video) = self.ivars().video.borrow().as_ref() { video.toggle(); }
            }
            return true;
        }
        match event.keyCode() {
            53 => {
                if self.ivars().previewing.get() {
                    self.focus_search();
                } else {
                    if !self.ivars().marked.borrow().is_empty() {
                        self.ivars().marked.borrow_mut().clear();
                        self.reload_marks();
                    } else {
                        self.hide_panel();
                    }
                }
                true
            }
            36 | 76 => {
                self.paste_selected();
                true
            }
            124 => {
                self.focus_preview();
                true
            }
            123 => {
                self.focus_search();
                true
            }
            125 => {
                if self.ivars().previewing.get() {
                    return false;
                }
                self.move_selection(1);
                if let Some(table) = self.ivars().table.borrow().as_ref() {
                    self.makeFirstResponder(Some(table));
                }
                true
            }
            126 => {
                if self.ivars().previewing.get() {
                    return false;
                }
                self.move_selection(-1);
                if let Some(table) = self.ivars().table.borrow().as_ref() {
                    self.makeFirstResponder(Some(table));
                }
                true
            }
            _ => false,
        }
    }
}

fn front_app() -> (String, String) {
    let Some(app) = NSWorkspace::sharedWorkspace().frontmostApplication() else {
        return (String::new(), String::new());
    };
    let name = app
        .localizedName()
        .map(|s| s.to_string())
        .unwrap_or_default();
    let bundle = app
        .bundleIdentifier()
        .map(|s| s.to_string())
        .unwrap_or_default();
    (name, bundle)
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(&input[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub fn run() {
    let mtm = MainThreadMarker::new().expect("main thread");
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    let main_menu = objc2_app_kit::NSMenu::new(mtm);
    let app_item = objc2_app_kit::NSMenuItem::new(mtm);
    let app_menu = objc2_app_kit::NSMenu::new(mtm);
    let settings_item = unsafe {
        app_menu.addItemWithTitle_action_keyEquivalent(
            ns_string!("Settings…"),
            Some(objc2::sel!(showSettings:)),
            ns_string!(","),
        )
    };
    unsafe {
        app_menu.addItemWithTitle_action_keyEquivalent(
            ns_string!("Quit Clipglass"),
            Some(objc2::sel!(terminate:)),
            ns_string!("q"),
        );
    }
    app_item.setSubmenu(Some(&app_menu));
    main_menu.addItem(&app_item);
    let edit_item = objc2_app_kit::NSMenuItem::new(mtm);
    edit_item.setTitle(ns_string!("Edit"));
    let edit_menu =
        objc2_app_kit::NSMenu::initWithTitle(objc2_app_kit::NSMenu::alloc(mtm), ns_string!("Edit"));
    for (title, action, key) in [
        ("Cut", objc2::sel!(cut:), "x"),
        ("Copy", objc2::sel!(copy:), "c"),
        ("Paste", objc2::sel!(paste:), "v"),
        ("Select All", objc2::sel!(selectAll:), "a"),
    ] {
        // A nil target routes these standard actions through the responder chain.
        unsafe {
            edit_menu.addItemWithTitle_action_keyEquivalent(
                &NSString::from_str(title),
                Some(action),
                &NSString::from_str(key),
            );
        }
    }
    edit_item.setSubmenu(Some(&edit_menu));
    main_menu.addItem(&edit_item);
    app.setMainMenu(Some(&main_menu));

    let data_dir = store::data_dir();
    let existing_install = data_dir.join("history.db").exists();
    let initial = if existing_install {
        crate::shortcuts::Config::legacy(paste::keycode_for_char('.').unwrap_or(47))
    } else {
        crate::shortcuts::Config::defaults(paste::keycode_for_char('v').unwrap_or(9))
    };
    let shortcuts = Rc::new(RefCell::new(
        crate::shortcuts::Service::open(data_dir.join("shortcuts.conf"), initial).unwrap_or_else(
            |error| {
                eprintln!("clip: shortcut initialization failed: {error}");
                std::process::exit(1);
            },
        ),
    ));
    let store = Store::open().unwrap_or_else(|e| {
        eprintln!("clip: {e}");
        std::process::exit(1);
    });
    let panel = ClipPanel::new(mtm, store, shortcuts.clone());
    unsafe {
        settings_item.setTarget(Some(&panel));
    }
    // Keep Settings discoverable even when a global shortcut is unavailable.
    let status = objc2_app_kit::NSStatusBar::systemStatusBar()
        .statusItemWithLength(objc2_app_kit::NSVariableStatusItemLength);
    if let Some(button) = status.button(mtm) {
        if let Some(image) = NSImage::imageWithSystemSymbolName_accessibilityDescription(
            ns_string!("doc.on.clipboard"),
            Some(ns_string!("Clipglass")),
        ) {
            image.setTemplate(true);
            button.setImage(Some(&image));
        } else {
            button.setTitle(ns_string!("Clipglass"));
        }
        button.setToolTip(Some(ns_string!("Clipglass")));
    }
    let status_menu = objc2_app_kit::NSMenu::new(mtm);
    for (title, selector, key) in [
        ("Open Clipboard History", objc2::sel!(showHistory:), ""),
        ("Open Images", objc2::sel!(showImages:), ""),
        ("Settings…", objc2::sel!(showSettings:), ","),
    ] {
        let item = unsafe {
            status_menu.addItemWithTitle_action_keyEquivalent(
                &NSString::from_str(title),
                Some(selector),
                &NSString::from_str(key),
            )
        };
        unsafe {
            item.setTarget(Some(&panel));
        }
    }
    unsafe {
        status_menu.addItemWithTitle_action_keyEquivalent(
            ns_string!("Quit Clipglass"),
            Some(objc2::sel!(terminate:)),
            ns_string!("q"),
        );
    }
    status.setMenu(Some(&status_menu));
    status.setVisible(crate::settings::menu_bar_icon_visible());
    eprintln!("clip: menu-bar icon visible: {}", status.isVisible());
    *panel.ivars().status_item.borrow_mut() = Some(status);
    if let Some(error) = shortcuts.borrow().issue.as_ref() {
        eprintln!("clip: {error}");
    }
    eprintln!(
        "clip: shortcuts loaded; paste keycode {}",
        paste::paste_keycode()
    );
    eprintln!(
        "clip: Accessibility trusted: {}",
        paste::accessibility_trusted()
    );

    let panel_for_keys = panel.clone();
    let block = RcBlock::new(move |event: std::ptr::NonNull<NSEvent>| -> *mut NSEvent {
        let event_ref = unsafe { event.as_ref() };
        let settings = panel_for_keys.ivars().settings.borrow().clone();
        if settings
            .as_ref()
            .is_some_and(|window| window.handle_key(event_ref))
        {
            return std::ptr::null_mut();
        }
        if crate::settings::is_settings_key(event_ref) {
            panel_for_keys.show_settings();
            return std::ptr::null_mut();
        }
        if panel_for_keys.handle_key(event_ref) {
            std::ptr::null_mut()
        } else {
            event.as_ptr()
        }
    });
    let block: &'static RcBlock<dyn Fn(std::ptr::NonNull<NSEvent>) -> *mut NSEvent> =
        Box::leak(Box::new(block));
    unsafe {
        NSEvent::addLocalMonitorForEventsMatchingMask_handler(NSEventMask::KeyDown, block);
    }

    unsafe {
        let _timer = NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
            0.05,
            &panel,
            objc2::sel!(tick:),
            None,
            true,
        );
        std::mem::forget(_timer);
    }

    if !existing_install || std::env::args().any(|arg| arg == "--show") {
        panel.show_panel();
    }
    app.run();
}
