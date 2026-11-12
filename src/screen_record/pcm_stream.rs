//! Keep PCM byte order across short writes and temporary encoder backpressure.
use std::{
    collections::VecDeque,
    io::{self, Write},
    time::{Duration, Instant},
};
const MAX_PENDING: usize = 8 * 1024 * 1024;
const STALL_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Default)]
pub(super) struct PendingPcm {
    bytes: VecDeque<u8>,
    last_progress: Option<Instant>,
}
impl PendingPcm {
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
    pub fn push(&mut self, bytes: &[u8]) -> io::Result<()> {
        if bytes.len() > MAX_PENDING.saturating_sub(self.bytes.len()) {
            return Err(io::Error::other("Encoder PCM queue exceeded 8 MiB"));
        }
        if self.bytes.is_empty() && !bytes.is_empty() {
            self.last_progress = Some(Instant::now());
        }
        self.bytes.extend(bytes);
        Ok(())
    }
    pub fn pump(&mut self, writer: &mut impl Write) -> io::Result<()> {
        self.pump_at(writer, Instant::now())
    }
    fn pump_at(&mut self, writer: &mut impl Write, now: Instant) -> io::Result<()> {
        if !self.is_empty()
            && self
                .last_progress
                .is_some_and(|last| now.duration_since(last) >= STALL_TIMEOUT)
        {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "Encoder did not accept PCM for 5 seconds",
            ));
        }
        let mut budget = 256 * 1024;
        while !self.is_empty() && budget > 0 {
            let bytes = self.bytes.as_slices().0;
            match writer.write(&bytes[..bytes.len().min(budget)]) {
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "Encoder closed PCM stream",
                    ));
                }
                Ok(count) => {
                    self.bytes.drain(..count);
                    budget -= count;
                    self.last_progress = Some(now);
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(()),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => return Ok(()),
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct SlowWriter {
        bytes: Vec<u8>,
        calls: usize,
    }
    impl Write for SlowWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.calls += 1;
            if self.calls % 2 == 0 {
                return Err(io::ErrorKind::WouldBlock.into());
            }
            let count = bytes.len().min(3);
            self.bytes.extend_from_slice(&bytes[..count]);
            Ok(count)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    #[test]
    fn resumes_partial_pcm_without_duplication_or_loss() {
        let mut queue = PendingPcm::default();
        let mut writer = SlowWriter {
            bytes: vec![],
            calls: 0,
        };
        queue.push(b"abcdefgh").unwrap();
        queue.pump(&mut writer).unwrap();
        queue.push(b"ijklmnop").unwrap();
        for _ in 0..10 {
            queue.pump(&mut writer).unwrap();
        }
        assert!(queue.is_empty());
        assert_eq!(writer.bytes, b"abcdefghijklmnop");
    }
    #[test]
    fn a_stalled_encoder_is_bounded() {
        let mut queue = PendingPcm::default();
        queue.push(b"pcm").unwrap();
        let now = queue.last_progress.unwrap() + STALL_TIMEOUT;
        assert_eq!(
            queue
                .pump_at(&mut Vec::<u8>::new(), now)
                .unwrap_err()
                .kind(),
            io::ErrorKind::TimedOut
        );
        assert!(queue.push(&vec![0; MAX_PENDING]).is_err());
    }
}
