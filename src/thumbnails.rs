//! Bounded, in-memory thumbnails. Decode on one worker, never full images on
//! the UI thread, and never write private screenshot thumbnails elsewhere.
use objc2::AnyThread;
use objc2::rc::Retained;
use objc2_app_kit::NSImage;
use objc2_core_foundation::{CFBoolean, CFDictionary, CFNumber, CFRetained, CFType, CFURL};
use objc2_core_graphics::CGImage;
use objc2_foundation::NSSize;
use objc2_image_io::{
    CGImageSource, kCGImageSourceCreateThumbnailFromImageAlways,
    kCGImageSourceCreateThumbnailWithTransform, kCGImageSourceShouldCache,
    kCGImageSourceShouldCacheImmediately, kCGImageSourceThumbnailMaxPixelSize,
};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, SyncSender};

pub const PIXELS: usize = 64;
const CAPACITY: usize = 128;

pub fn decode(path: &Path) -> Option<CFRetained<CGImage>> {
    let url = CFURL::from_file_path(path)?;
    let source_options = CFDictionary::from_slices(
        &[unsafe { kCGImageSourceShouldCache }],
        &[CFBoolean::new(false)],
    );
    let source = unsafe { CGImageSource::with_url(&url, Some(source_options.as_opaque())) }?;
    let size = CFNumber::new_isize(PIXELS as isize);
    let yes = CFBoolean::new(true);
    let values: [&CFType; 4] = [yes, yes, &size, yes];
    let options = CFDictionary::from_slices(
        &[
            unsafe { kCGImageSourceCreateThumbnailFromImageAlways },
            unsafe { kCGImageSourceCreateThumbnailWithTransform },
            unsafe { kCGImageSourceThumbnailMaxPixelSize },
            unsafe { kCGImageSourceShouldCacheImmediately },
        ],
        &values,
    );
    // ImageIO options contain exactly the CFBoolean/CFNumber values above.
    unsafe { source.thumbnail_at_index(0, Some(options.as_opaque())) }
}

struct Completed {
    path: PathBuf,
    image: Option<CFRetained<CGImage>>,
}

pub struct Thumbnails {
    sender: SyncSender<PathBuf>,
    results: Receiver<Completed>,
    pending: HashSet<PathBuf>,
    cache: HashMap<PathBuf, Option<Retained<NSImage>>>,
    age: VecDeque<PathBuf>,
}

impl Thumbnails {
    pub fn new() -> Self {
        let (sender, jobs) = mpsc::sync_channel::<PathBuf>(64);
        let (output, results) = mpsc::sync_channel(64);
        std::thread::Builder::new()
            .name("clip-thumbnails".into())
            .spawn(move || {
                while let Ok(path) = jobs.recv() {
                    let image = decode(&path);
                    if output.send(Completed { path, image }).is_err() {
                        break;
                    }
                }
            })
            .expect("thumbnail worker");
        Self {
            sender,
            results,
            pending: HashSet::new(),
            cache: HashMap::new(),
            age: VecDeque::new(),
        }
    }

    pub fn get(&mut self, path: &Path) -> Option<Retained<NSImage>> {
        if let Some(image) = self.cache.get(path) {
            self.age.retain(|key| key != path);
            self.age.push_back(path.to_path_buf());
            return image.clone();
        }
        if !self.pending.contains(path) && self.sender.try_send(path.to_path_buf()).is_ok() {
            self.pending.insert(path.to_path_buf());
        }
        None
    }

    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        while let Ok(completed) = self.results.try_recv() {
            self.pending.remove(&completed.path);
            let image = completed
                .image
                .map(|image| NSImage::initWithCGImage_size(NSImage::alloc(), &image, NSSize::ZERO));
            self.age.retain(|key| key != &completed.path);
            self.age.push_back(completed.path.clone());
            self.cache.insert(completed.path, image);
            while self.cache.len() > CAPACITY {
                if let Some(old) = self.age.pop_front() {
                    self.cache.remove(&old);
                }
            }
            changed = true;
        }
        changed
    }
}
