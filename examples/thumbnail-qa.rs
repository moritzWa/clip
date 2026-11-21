//! VM-only ImageIO thumbnail and background-delivery check, synthetic image only.
#[path = "../src/thumbnails.rs"]
mod thumbnails;
fn main() {
    assert_eq!(std::env::var("CLIP_VM_TEST").as_deref(), Ok("1"));
    let path = std::path::PathBuf::from(std::env::args().nth(1).expect("synthetic image path"));
    let decoded = thumbnails::decode(&path).expect("synthetic thumbnail");
    let width = objc2_core_graphics::CGImage::width(Some(&decoded));
    let height = objc2_core_graphics::CGImage::height(Some(&decoded));
    assert!(width > 0 && height > 0);
    assert!(width <= thumbnails::PIXELS && height <= thumbnails::PIXELS);
    assert!(
        thumbnails::decode(std::path::Path::new("/nonexistent-thumbnail-fixture.png")).is_none()
    );
    let mut cache = thumbnails::Thumbnails::new();
    assert!(cache.get(&path).is_none());
    for _ in 0..500 {
        if cache.poll() {
            let image = cache
                .get(&path)
                .expect("worker must deliver the decoded image");
            assert!(image.size().width <= 64.0 && image.size().height <= 64.0);
            println!(
                "PASS: {}×{} ImageIO thumbnail, asynchronous delivery and cache hit",
                width, height
            );
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("thumbnail worker did not deliver within 5 seconds");
}
