use std::ffi::c_void;

pub fn accessibility_trusted() -> bool {
    unsafe { AXIsProcessTrusted() }
}

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> bool;
}

/// Keycode whose unmodified output is `v` in the current layout.
/// ADNW maps that to physical Y, not V. Posting physical V pastes a comma.
pub fn paste_keycode() -> u16 {
    keycode_for_char('v').unwrap_or(16)
}

pub fn post_paste() {
    if !accessibility_trusted() {
        eprintln!("clip: item copied; automatic paste needs Accessibility permission for Clip.app");
        return;
    }
    let code = paste_keycode();
    unsafe {
        let down = CGEventCreateKeyboardEvent(std::ptr::null_mut(), code, true);
        let up = CGEventCreateKeyboardEvent(std::ptr::null_mut(), code, false);
        if down.is_null() || up.is_null() {
            return;
        }
        CGEventSetFlags(down, kCGEventFlagMaskCommand);
        CGEventSetFlags(up, kCGEventFlagMaskCommand);
        CGEventPost(kCGHIDEventTap, down);
        CGEventPost(kCGHIDEventTap, up);
        CFRelease(down);
        CFRelease(up);
    }
}

const kCGHIDEventTap: u32 = 0;
const kCGEventFlagMaskCommand: u64 = 0x100000;
const kUCKeyActionDisplay: u16 = 3;
const kUCKeyTranslateNoDeadKeysBit: u32 = 1;

pub fn keycode_for_char(want: char) -> Option<u16> {
    unsafe {
        let source = TISCopyCurrentKeyboardInputSource();
        if source.is_null() {
            return None;
        }
        let data = TISGetInputSourceProperty(source, kTISPropertyUnicodeKeyLayoutData);
        if data.is_null() {
            CFRelease(source);
            return None;
        }
        let layout = CFDataGetBytePtr(data);
        if layout.is_null() {
            CFRelease(source);
            return None;
        }
        let keyboard_type = LMGetKbdType() as u32;
        for code in 0..128u16 {
            let mut dead = 0u32;
            let mut len = 0usize;
            let mut chars = [0u16; 4];
            let rc = UCKeyTranslate(
                layout,
                code,
                kUCKeyActionDisplay,
                0,
                keyboard_type,
                kUCKeyTranslateNoDeadKeysBit,
                &mut dead,
                chars.len(),
                &mut len,
                chars.as_mut_ptr(),
            );
            if rc == 0 && len == 1 && chars[0] == want as u16 {
                CFRelease(source);
                return Some(code);
            }
        }
        CFRelease(source);
        None
    }
}

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    static kTISPropertyUnicodeKeyLayoutData: *const c_void;
    fn TISCopyCurrentKeyboardInputSource() -> *mut c_void;
    fn TISGetInputSourceProperty(source: *mut c_void, key: *const c_void) -> *mut c_void;
    fn LMGetKbdType() -> u8;
    fn UCKeyTranslate(
        layout: *const u8,
        code: u16,
        action: u16,
        modifier_state: u32,
        keyboard_type: u32,
        options: u32,
        dead_key_state: *mut u32,
        max_len: usize,
        actual_len: *mut usize,
        unicode: *mut u16,
    ) -> i32;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFDataGetBytePtr(data: *const c_void) -> *const u8;
    fn CFRelease(cf: *const c_void);
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventCreateKeyboardEvent(
        source: *mut c_void,
        virtual_key: u16,
        key_down: bool,
    ) -> *mut c_void;
    fn CGEventSetFlags(event: *mut c_void, flags: u64);
    fn CGEventPost(tap: u32, event: *mut c_void);
}
