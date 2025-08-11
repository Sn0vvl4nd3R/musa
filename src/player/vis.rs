use std::collections::VecDeque;
use std::sync::{
    Arc,
    Mutex
};

pub struct VisBuffer {
    q: Mutex<VecDeque<f32>>,
    cap: usize,
}

impl VisBuffer {
    pub fn new(cap: usize) -> Arc<Self> {
        Arc::new(Self {
            q: Mutex::new(VecDeque::with_capacity(cap)),
            cap
        })
    }
    pub fn push(&self, s: f32) {
        let mut q = self.q.lock().unwrap();
        let len = q.len();
        if len >= self.cap {
            q.drain(..len + 1 - self.cap);
        }
        q.push_back(s);
    }
    pub fn take_recent(&self, n: usize) -> Vec<f32> {
        let q = self.q.lock().unwrap();
        let k = n.min(q.len());
        q.iter().rev().take(k).cloned().collect::<Vec<_>>().into_iter().rev().collect()
    }
}
