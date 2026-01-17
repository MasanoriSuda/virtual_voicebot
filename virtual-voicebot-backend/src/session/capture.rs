use std::time::{Duration, Instant};

pub struct AudioCapture {
    window: Duration,
    started_at: Option<Instant>,
    payloads: Vec<u8>,
}

impl AudioCapture {
    pub fn new(window: Duration) -> Self {
        Self {
            window,
            started_at: None,
            payloads: Vec::new(),
        }
    }

    pub fn start(&mut self) {
        self.started_at = Some(Instant::now());
        self.payloads.clear();
    }

    pub fn reset(&mut self) {
        self.started_at = None;
        self.payloads.clear();
    }

    pub fn ingest(&mut self, payload: &[u8]) -> Option<Vec<u8>> {
        let Some(start) = self.started_at else {
            return None;
        };

        self.payloads.extend_from_slice(payload);
        if start.elapsed() >= self.window {
            let mut out = Vec::new();
            std::mem::swap(&mut out, &mut self.payloads);
            self.started_at = None;
            return Some(out);
        }
        None
    }
}
