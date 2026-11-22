//! VM-only native clipboard regression. Uses a unique pasteboard, never the
//! user's global clipboard. Arguments are generated PNG and JPEG fixtures.
#[path = "../src/clipboard.rs"]
mod clipboard;

use objc2::AnyThread;
use objc2_app_kit::{NSImage, NSPasteboard};
use objc2_foundation::{NSArray, NSData, NSString};

fn main() {
    assert_eq!(std::env::var("CLIP_VM_TEST").as_deref(), Ok("1"));
    let args: Vec<_> = std::env::args().collect();
    let png = NSData::from_vec(std::fs::read(&args[1]).unwrap());
    let jpeg = NSData::from_vec(std::fs::read(&args[2]).unwrap());
    let pb = NSPasteboard::pasteboardWithUniqueName();
    let text_type = NSString::from_str("public.utf8-plain-text");
    let png_type = NSString::from_str("public.png");
    let jpeg_type = NSString::from_str("public.jpeg");
    unsafe {
        pb.declareTypes_owner(
            &NSArray::from_retained_slice(&[text_type.clone(), png_type.clone()]),
            None,
        );
    }
    assert!(pb.setString_forType(&NSString::from_str("synthetic browser caption"), &text_type));
    let count = pb.changeCount();
    let mut capture = clipboard::Capture::new(count - 1);
    assert!(capture.begin(count, Some("Synthetic browser".into())));
    let first = capture.read(&pb);
    assert_eq!(first.text.as_deref(), Some("synthetic browser caption"));
    assert!(first.image.is_none());
    for _ in 0..8 {
        assert!(capture.begin(count, None));
        let pending = capture.read(&pb);
        assert!(pending.text.is_none() && pending.image.is_none());
    }
    // Fulfill the advertised image after several unsuccessful reads. This
    // updates promised data, but does NOT declare a new clipboard change.
    assert!(pb.setData_forType(Some(&png), &png_type));
    assert_eq!(pb.changeCount(), count);
    assert!(capture.begin(count, None));
    let late = capture.read(&pb);
    assert!(late.text.is_none());
    assert_eq!(late.image.unwrap().1, "png");
    assert!(!capture.begin(count, None));
    println!("PASS: image captured after eight empty reads without another copy or any paste");

    unsafe {
        pb.declareTypes_owner(&NSArray::from_retained_slice(&[jpeg_type.clone()]), None);
    }
    assert!(pb.setData_forType(Some(&jpeg), &jpeg_type));
    assert!(capture.begin(pb.changeCount(), Some("Synthetic browser".into())));
    let (bytes, ext) = capture
        .read(&pb)
        .image
        .expect("JPEG-only clipboard must capture");
    assert_eq!(ext, "tiff");
    assert!(NSImage::initWithData(NSImage::alloc(), &NSData::from_vec(bytes)).is_some());
    println!("PASS: JPEG-only image normalized to a decodable native TIFF");
    pb.clearContents();
}
