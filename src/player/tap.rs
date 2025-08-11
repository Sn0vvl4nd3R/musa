use rodio::Source;
use std::sync::Arc;
use super::vis::VisBuffer;

pub(super) struct TapSource<S> {
    inner: S,
    buf: Arc<VisBuffer>,
}

impl<S> TapSource<S> {
    pub(super) fn new(inner: S, buf: Arc<VisBuffer>) -> Self {
        Self {
            inner,
            buf
        }
    }
}

impl<S> Iterator for TapSource<S>
where
    S: Source<Item = f32>,
{
    type Item = f32;
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|x| {
            self.buf.push(x);
            x
        })
    }
}

impl<S> Source for TapSource<S>
where
    S: Source<Item = f32>,
{
    fn current_frame_len(&self) -> Option<usize> {
        self.inner.current_frame_len()
    }
    fn channels(&self) -> u16 {
        self.inner.channels()
    }
    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }
    fn total_duration(&self) -> Option<std::time::Duration> {
        self.inner.total_duration()
    }
}
