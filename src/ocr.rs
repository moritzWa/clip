//! One background Vision worker. Only paths and UTF-8 text cross the thread boundary.
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};

use objc2::AnyThread;
use objc2::rc::autoreleasepool;
use objc2_foundation::{NSArray, NSDictionary, NSString, NSURL};
use objc2_vision::{
    VNImageRequestHandler, VNRecognizeTextRequest, VNRequest, VNRequestTextRecognitionLevel,
};

pub struct Job {
    pub id: String,
    pub path: PathBuf,
}

pub struct Completed {
    pub id: String,
    pub text: Result<String, String>,
}

pub struct Worker {
    jobs: SyncSender<Job>,
    pub results: Receiver<Completed>,
}

impl Worker {
    pub fn new() -> Self {
        let (jobs, incoming) = mpsc::sync_channel::<Job>(8);
        let (outgoing, results) = mpsc::sync_channel(8);
        std::thread::Builder::new()
            .name("clip-ocr".into())
            .spawn(move || {
                while let Ok(job) = incoming.recv() {
                    let text = autoreleasepool(|_| recognize(&job.path));
                    if outgoing.send(Completed { id: job.id, text }).is_err() {
                        break;
                    }
                }
            })
            .expect("start OCR worker");
        Self { jobs, results }
    }

    pub fn submit(&self, job: Job) -> Result<(), TrySendError<Job>> {
        self.jobs.try_send(job)
    }
}

fn recognize(path: &std::path::Path) -> Result<String, String> {
    let url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
    let request = VNRecognizeTextRequest::new();
    request.setRecognitionLevel(VNRequestTextRecognitionLevel::Accurate);
    request.setUsesLanguageCorrection(true);
    request.setAutomaticallyDetectsLanguage(true);
    let handler = unsafe {
        // An empty options dictionary cannot contain incorrectly typed values.
        VNImageRequestHandler::initWithURL_options(
            VNImageRequestHandler::alloc(),
            &url,
            &NSDictionary::new(),
        )
    };
    let requests = NSArray::<VNRequest>::from_slice(&[&request]);
    handler
        .performRequests_error(&requests)
        .map_err(|e| e.to_string())?;
    let mut lines = Vec::new();
    if let Some(observations) = request.results() {
        for observation in observations.iter() {
            if let Some(best) = observation.topCandidates(1).firstObject() {
                lines.push(best.string().to_string());
            }
        }
    }
    Ok(lines.join("\n"))
}
