//! Read native clipboard payloads, including browser-provided/lazy images.
//! No URLs are fetched and no source contents are logged.
use objc2::{AnyThread, rc::Retained};
use objc2_app_kit::{NSImage, NSPasteboard};
use objc2_foundation::{NSData, NSString};

const MAX_ATTEMPTS: u8 = 60; // Three seconds at the panel's 50 ms polling interval.
const SETTLE_ATTEMPTS: u8 = 5;
const IMAGE_TYPES: &[(&str, &str)] = &[
    ("public.png", "png"),
    ("public.tiff", "tiff"),
    ("public.jpeg", "jpeg"),
    ("com.compuserve.gif", "gif"),
    ("org.webmproject.webp", "webp"),
    ("public.webp", "webp"),
    ("public.heic", "heic"),
    ("public.heif", "heif"),
    ("public.avif", "avif"),
    ("com.microsoft.bmp", "bmp"),
];

#[derive(Default)]
pub struct Capture {
    change: Option<isize>,
    attempts: u8,
    text_done: bool,
    image_done: bool,
    file_done: bool,
    expects_image: bool,
    pub source: Option<String>,
}

#[derive(Default)]
pub struct Payload {
    pub text: Option<String>,
    pub image: Option<(Vec<u8>, &'static str)>,
    pub file_url: Option<String>,
}

impl Capture {
    pub fn new(change: isize) -> Self {
        Self {
            change: Some(change),
            attempts: MAX_ATTEMPTS,
            ..Self::default()
        }
    }

    // Keeps partial successes while retrying promised data. Previously one
    // empty read consumed changeCount forever, even when the browser filled it
    // later without another change notification.
    pub fn begin(&mut self, change: isize, source: Option<String>) -> bool {
        if self.change != Some(change) {
            *self = Self {
                change: Some(change),
                source,
                ..Self::default()
            };
        }
        if self.attempts >= MAX_ATTEMPTS {
            return false;
        }
        if self.attempts >= SETTLE_ATTEMPTS
            && (self.text_done || self.image_done || self.file_done)
            && (!self.expects_image || self.image_done)
        {
            return false;
        }
        self.attempts += 1;
        true
    }

    pub fn read(&mut self, pb: &NSPasteboard) -> Payload {
        let mut payload = Payload::default();
        let types: Vec<String> = pb
            .types()
            .map(|types| types.iter().map(|t| t.to_string()).collect())
            .unwrap_or_default();
        self.expects_image |= IMAGE_TYPES
            .iter()
            .any(|(name, _)| types.iter().any(|t| t == name));
        if !self.text_done {
            if let Some(text) = pb.stringForType(&NSString::from_str("public.utf8-plain-text")) {
                let text = text.to_string();
                // Browser address bars copy local URLs as plain text, not as
                // native files. Suppress the duplicate URL only when the
                // pasteboard actually advertises a native file representation.
                let native_file_url =
                    text.starts_with("file://") && types.iter().any(|t| t == "public.file-url");
                if !text.is_empty() && !native_file_url {
                    self.text_done = true;
                    payload.text = Some(text);
                }
            }
        }
        if !self.image_done {
            for &(name, ext) in IMAGE_TYPES {
                let Some(data) = pb.dataForType(&NSString::from_str(name)) else {
                    continue;
                };
                if data.length() == 0 || data.length() > 40_000_000 {
                    continue;
                }
                // Validate the representation. Convert non-PNG/TIFF formats to
                // TIFF so storage and native pasting retain their existing types.
                let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) else {
                    continue;
                };
                let normalized: Option<Retained<NSData>> = if matches!(ext, "png" | "tiff") {
                    Some(data)
                } else {
                    image.TIFFRepresentation()
                };
                if let Some(data) =
                    normalized.filter(|d| d.length() > 0 && d.length() <= 40_000_000)
                {
                    self.image_done = true;
                    payload.image =
                        Some((data.to_vec(), if ext == "png" { "png" } else { "tiff" }));
                    break;
                }
            }
        }
        if !self.file_done {
            if let Some(url) = pb.stringForType(&NSString::from_str("public.file-url")) {
                let url = url.to_string();
                if !url.is_empty() {
                    self.file_done = true;
                    payload.file_url = Some(url);
                }
            }
        }
        payload
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use objc2_foundation::NSArray;
    // AppKit pasteboard creation can alias during concurrent calls in one process.
    // Keep native pasteboard tests isolated from one another as well as the user.
    static PASTEBOARD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn consecutive_local_file_urls_copied_as_text_are_captured() {
        let _guard = PASTEBOARD_LOCK.lock().unwrap();
        // A unique native pasteboard reproduces Chrome's address-bar copy
        // without reading or changing the user's general clipboard.
        let pb = NSPasteboard::pasteboardWithUniqueName();
        let text_type = NSString::from_str("public.utf8-plain-text");
        let mut capture = Capture::new(pb.changeCount());
        for url in [
            "file:///synthetic/articles/first%20article.html",
            "file:///synthetic/articles/second-article.html",
        ] {
            unsafe {
                pb.declareTypes_owner(&NSArray::from_retained_slice(&[text_type.clone()]), None);
            }
            assert!(pb.setString_forType(&NSString::from_str(url), &text_type));
            assert!(capture.begin(pb.changeCount(), Some("Synthetic browser".into())));
            let payload = capture.read(&pb);
            assert_eq!(payload.text.as_deref(), Some(url));
            assert!(payload.file_url.is_none());
            assert!(capture.begin(pb.changeCount(), None));
            assert!(
                capture.read(&pb).text.is_none(),
                "do not capture the same URL twice"
            );
        }
        pb.clearContents();
    }

    #[test]
    fn native_file_copy_does_not_duplicate_its_url_as_text() {
        let _guard = PASTEBOARD_LOCK.lock().unwrap();
        let pb = NSPasteboard::pasteboardWithUniqueName();
        let text_type = NSString::from_str("public.utf8-plain-text");
        let file_type = NSString::from_str("public.file-url");
        let url = NSString::from_str("file:///synthetic/example.txt");
        unsafe {
            pb.declareTypes_owner(
                &NSArray::from_retained_slice(&[text_type.clone(), file_type.clone()]),
                None,
            );
        }
        assert!(pb.setString_forType(&url, &text_type));
        assert!(pb.setString_forType(&url, &file_type));
        let mut capture = Capture::new(pb.changeCount() - 1);
        for _ in 0..SETTLE_ATTEMPTS {
            assert!(capture.begin(pb.changeCount(), None));
            let payload = capture.read(&pb);
            assert!(payload.text.is_none());
        }
        assert!(capture.file_done);
        pb.clearContents();
    }

    #[test]
    fn promised_images_are_retried_after_text_without_recapturing_text() {
        let mut capture = Capture::new(0);
        assert!(capture.begin(1, Some("Browser".into())));
        capture.text_done = true;
        capture.expects_image = true;
        for _ in 0..20 {
            assert!(capture.begin(1, Some("Clipglass".into())));
            assert!(capture.text_done);
            assert_eq!(capture.source.as_deref(), Some("Browser"));
        }
        capture.image_done = true;
        assert!(!capture.begin(1, None));
    }

    #[test]
    fn empty_reads_are_bounded_and_new_copies_reset_pending_work() {
        let mut capture = Capture::new(0);
        assert!(!capture.begin(0, None));
        for _ in 0..MAX_ATTEMPTS {
            assert!(capture.begin(1, None));
        }
        assert!(!capture.begin(1, None));
        assert!(capture.begin(2, Some("Browser".into())));
        assert!(!capture.text_done && !capture.image_done && !capture.file_done);
    }
}
