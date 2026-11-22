//! Native shortcut settings. No webview and no global keyboard-event tap.
use crate::{
    paste,
    shortcuts::{self, Config, Service, Shortcut},
};
use objc2::runtime::NSObjectProtocol;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained};
use objc2_app_kit::*;
use objc2_foundation::{NSNotification, NSPoint, NSRect, NSSize, NSString, ns_string};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

pub struct Ivars {
    service: Rc<RefCell<Service>>,
    recording: Cell<Option<usize>>,
    buttons: RefCell<Vec<Retained<NSButton>>>,
    status: RefCell<Option<Retained<NSTextField>>>,
}
define_class!(
    #[unsafe(super(NSWindow))]
    #[thread_kind=MainThreadOnly]
    #[ivars=Ivars]
    pub struct SettingsWindow;
    unsafe impl NSObjectProtocol for SettingsWindow {}
    unsafe impl NSWindowDelegate for SettingsWindow {
        #[unsafe(method(windowWillClose:))]
        fn will_close(&self,_:&NSNotification) {self.cancel_recording();}
        #[unsafe(method(windowDidResignKey:))]
        fn resigned(&self,_:&NSNotification) {self.cancel_recording();}
    }
    impl SettingsWindow {
        #[unsafe(method(recordHistory:))]
        fn record_history(&self,_:&NSButton) {self.start_recording(0);}
        #[unsafe(method(recordImages:))]
        fn record_images(&self,_:&NSButton) {self.start_recording(1);}
        #[unsafe(method(restoreDefaults:))]
        fn restore(&self,_:&NSButton) {
            self.cancel_recording();
            let config=Config::defaults(paste::keycode_for_char('v').unwrap_or(9));
            let result=self.ivars().service.borrow_mut().apply(config);
            match result {Ok(())=>self.set_status("Defaults restored."),Err(error)=>self.set_status(&error)}
            self.update_buttons();
        }
        #[unsafe(method(done:))]
        fn done(&self,_:&NSButton) {self.close();}
    }
);

fn rect(x: f64, y: f64, w: f64, h: f64) -> NSRect {
    NSRect::new(NSPoint::new(x, y), NSSize::new(w, h))
}
fn label(text: &str, size: f64, frame: NSRect, mtm: MainThreadMarker) -> Retained<NSTextField> {
    let label = NSTextField::labelWithString(&NSString::from_str(text), mtm);
    label.setFont(Some(&NSFont::systemFontOfSize(size)));
    label.setFrame(frame);
    label
}

impl SettingsWindow {
    pub fn new(mtm: MainThreadMarker, service: Rc<RefCell<Service>>) -> Retained<Self> {
        let window = Self::alloc(mtm).set_ivars(Ivars {
            service,
            recording: Cell::new(None),
            buttons: RefCell::new(Vec::new()),
            status: RefCell::new(None),
        });
        let window: Retained<Self> = unsafe {
            msg_send![super(window),initWithContentRect:rect(0.,0.,540.,330.),styleMask:NSWindowStyleMask::Titled|NSWindowStyleMask::Closable,backing:NSBackingStoreType::Buffered,defer:false]
        };
        unsafe {
            window.setReleasedWhenClosed(false);
        }
        window.setTitle(ns_string!("Clipglass Settings"));
        window.setDelegate(Some(objc2::runtime::ProtocolObject::from_ref(&*window)));
        let content = NSView::initWithFrame(NSView::alloc(mtm), rect(0., 0., 540., 330.));
        content.addSubview(&label(
            "Keyboard shortcuts",
            21.,
            rect(28., 273., 484., 30.),
            mtm,
        ));
        for (index, (title, selector, y)) in [
            ("Open clipboard history", objc2::sel!(recordHistory:), 219.),
            (
                "Open images / toggle filter",
                objc2::sel!(recordImages:),
                171.,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            content.addSubview(&label(title, 14., rect(28., y, 270., 24.), mtm));
            let button = unsafe {
                NSButton::buttonWithTitle_target_action(
                    ns_string!(""),
                    Some(&window),
                    Some(selector),
                    mtm,
                )
            };
            button.setFrame(rect(330., y - 5., 182., 34.));
            button.setToolTip(Some(&NSString::from_str(if index == 0 {
                "Record the global history shortcut"
            } else {
                "Record the global image shortcut"
            })));
            content.addSubview(&button);
            window.ivars().buttons.borrow_mut().push(button);
        }
        let help = label(
            "Click a shortcut, then press a new combination. Esc cancels.",
            12.,
            rect(28., 126., 484., 20.),
            mtm,
        );
        help.setTextColor(Some(&NSColor::secondaryLabelColor()));
        content.addSubview(&help);
        let hint = label(
            "Keyboard remappers may translate the combination shown here.",
            11.,
            rect(28., 105., 484., 18.),
            mtm,
        );
        hint.setTextColor(Some(&NSColor::secondaryLabelColor()));
        content.addSubview(&hint);
        let status = label("", 12., rect(28., 59., 484., 39.), mtm);
        content.addSubview(&status);
        *window.ivars().status.borrow_mut() = Some(status);
        let defaults = unsafe {
            NSButton::buttonWithTitle_target_action(
                ns_string!("Restore Defaults"),
                Some(&window),
                Some(objc2::sel!(restoreDefaults:)),
                mtm,
            )
        };
        defaults.setFrame(rect(23., 18., 155., 32.));
        content.addSubview(&defaults);
        let done = unsafe {
            NSButton::buttonWithTitle_target_action(
                ns_string!("Done"),
                Some(&window),
                Some(objc2::sel!(done:)),
                mtm,
            )
        };
        done.setFrame(rect(414., 18., 103., 32.));
        done.setKeyEquivalent(ns_string!("\r"));
        content.addSubview(&done);
        window.setContentView(Some(&content));
        window.center();
        window.update_buttons();
        window
    }

    pub fn show(&self) {
        self.update_buttons();
        if let Some(error) = self.ivars().service.borrow().issue.as_ref() {
            self.set_status(error);
        }
        self.makeKeyAndOrderFront(None);
        NSApplication::sharedApplication(self.mtm()).activateIgnoringOtherApps(true);
    }
    fn set_status(&self, text: &str) {
        if let Some(status) = self.ivars().status.borrow().as_ref() {
            status.setStringValue(&NSString::from_str(text));
        }
    }
    fn update_buttons(&self) {
        let config = self.ivars().service.borrow().config;
        for (index, (button, shortcut)) in self
            .ivars()
            .buttons
            .borrow()
            .iter()
            .zip([config.history, config.images])
            .enumerate()
        {
            let text = if self.ivars().recording.get() == Some(index) {
                "Press shortcut…".into()
            } else {
                display(shortcut)
            };
            button.setTitle(&NSString::from_str(&text));
        }
    }
    fn start_recording(&self, index: usize) {
        if self.ivars().recording.get().is_none() {
            let result = self.ivars().service.borrow_mut().suspend();
            if let Err(error) = result {
                self.set_status(&error);
                return;
            }
        }
        self.ivars().recording.set(Some(index));
        self.set_status("Press a combination with Command or Control. Esc cancels.");
        self.update_buttons();
    }
    fn cancel_recording(&self) {
        if self.ivars().recording.take().is_some() {
            let result = self.ivars().service.borrow_mut().resume();
            match result {
                Ok(()) => self.set_status("Shortcut unchanged."),
                Err(error) => self.set_status(&error),
            }
            self.update_buttons();
        }
    }
    pub fn handle_key(&self, event: &NSEvent) -> bool {
        let Some(index) = self.ivars().recording.get() else {
            return false;
        };
        if !self.isVisible() || !self.isKeyWindow() {
            return false;
        }
        if event.keyCode() == 53 {
            self.cancel_recording();
            return true;
        }
        if event.isARepeat() {
            return true;
        }
        let modifiers = event_modifiers(event);
        if let Some(chars) = event.charactersIgnoringModifiers() {
            let chars = chars.to_string().to_lowercase();
            if modifiers == shortcuts::COMMAND && (chars == "," || chars == "q") {
                self.set_status("Command-comma and Command-Q are reserved for Settings and Quit.");
                return true;
            }
        }
        let shortcut = Shortcut {
            key: event.keyCode(),
            modifiers,
        };
        let mut config = self.ivars().service.borrow().config;
        if index == 0 {
            config.history = shortcut;
        } else {
            config.images = shortcut;
        }
        let result = self.ivars().service.borrow_mut().apply(config);
        match result {
            Ok(()) => {
                self.ivars().recording.set(None);
                self.set_status("Shortcut saved.");
                self.update_buttons();
            }
            Err(error) => self.set_status(&error),
        }
        true
    }
}

pub fn event_modifiers(event: &NSEvent) -> u8 {
    let flags = event.modifierFlags();
    let mut result = 0;
    for (native, stored) in [
        (NSEventModifierFlags::Command, shortcuts::COMMAND),
        (NSEventModifierFlags::Shift, shortcuts::SHIFT),
        (NSEventModifierFlags::Control, shortcuts::CONTROL),
        (NSEventModifierFlags::Option, shortcuts::OPTION),
    ] {
        if flags.contains(native) {
            result |= stored;
        }
    }
    result
}
pub fn is_settings_key(event: &NSEvent) -> bool {
    event_modifiers(event) == shortcuts::COMMAND
        && event
            .charactersIgnoringModifiers()
            .is_some_and(|s| s.to_string() == ",")
}
fn display(shortcut: Shortcut) -> String {
    let mut label = String::new();
    for (flag, symbol) in [
        (shortcuts::CONTROL, "⌃"),
        (shortcuts::OPTION, "⌥"),
        (shortcuts::SHIFT, "⇧"),
        (shortcuts::COMMAND, "⌘"),
    ] {
        if shortcut.modifiers & flag != 0 {
            label.push_str(symbol);
        }
    }
    let key = match shortcut.key {
        36 | 76 => "↩".into(),
        48 => "⇥".into(),
        49 => "Space".into(),
        51 => "⌫".into(),
        117 => "⌦".into(),
        123 => "←".into(),
        124 => "→".into(),
        125 => "↓".into(),
        126 => "↑".into(),
        key => paste::key_character(key)
            .filter(|s| {
                s.chars()
                    .all(|c| !c.is_control() && !(0xf700..=0xf8ff).contains(&(c as u32)))
            })
            .map(|s| s.to_uppercase())
            .unwrap_or_else(|| format!("{:?}", shortcuts::native_code(key).unwrap())),
    };
    label.push_str(&key);
    label
}
