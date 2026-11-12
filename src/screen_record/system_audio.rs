//! WASAPI output loopback, streamed as bounded raw PCM to the encoder.
//! No microphone endpoint, temporary audio file, or virtual audio driver.
use std::{
    net::{Shutdown, TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};

pub(super) struct Session {
    pub rate: u32,
    pub channels: u16,
    pub address: String,
    stop: Arc<AtomicBool>,
    socket: Arc<Mutex<Option<TcpStream>>>,
    error: Arc<Mutex<Option<String>>>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Session {
    pub fn start(trace: &super::diagnostics::Trace) -> Result<Self, String> {
        let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|e| e.to_string())?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let address = format!(
            "tcp://{}",
            listener.local_addr().map_err(|e| e.to_string())?
        );
        let stop = Arc::new(AtomicBool::new(false));
        let socket = Arc::new(Mutex::new(None));
        let error = Arc::new(Mutex::new(None));
        let (tx, rx) = mpsc::sync_channel(1);
        let (s, sock, err) = (stop.clone(), socket.clone(), error.clone());
        let trace = trace.clone();
        let thread = std::thread::spawn(move || {
            if let Err(message) = capture(listener, &s, &sock, &tx, &trace) {
                trace.note("audio.failed", &message);
                *err.lock().unwrap() = Some(message);
            }
        });
        let mut session = Self {
            rate: 0,
            channels: 0,
            address,
            stop,
            socket,
            error,
            thread: Some(thread),
        };
        let format = rx.recv_timeout(Duration::from_secs(5)).map_err(|_| {
            session
                .error()
                .unwrap_or_else(|| "System audio initialization timed out".into())
        })?;
        session.rate = format.0;
        session.channels = format.1;
        Ok(session)
    }
    pub fn error(&self) -> Option<String> {
        self.error.lock().unwrap().clone()
    }
    pub fn finish(&mut self) {
        // Drain the short playout buffer before asking the encoder to finalize.
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        if let Some(socket) = self.socket.lock().unwrap().as_ref() {
            let _ = socket.shutdown(Shutdown::Write);
        }
    }
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Release);
        if let Some(socket) = self.socket.lock().unwrap().as_ref() {
            let _ = socket.shutdown(Shutdown::Both);
        }
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.stop();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(not(windows))]
fn capture(
    _: TcpListener,
    _: &AtomicBool,
    _: &Mutex<Option<TcpStream>>,
    _: &mpsc::SyncSender<(u32, u16)>,
    _: &super::diagnostics::Trace,
) -> Result<(), String> {
    Err("System audio capture requires Windows".into())
}

#[cfg(windows)]
fn capture(
    listener: TcpListener,
    stop: &AtomicBool,
    socket: &Mutex<Option<TcpStream>>,
    ready: &mpsc::SyncSender<(u32, u16)>,
    trace: &super::diagnostics::Trace,
) -> Result<(), String> {
    use std::{collections::VecDeque, time::Instant};
    use windows::Win32::{
        Media::Audio::*,
        System::{Com::*, Performance::*},
    };
    struct Com;
    impl Drop for Com {
        fn drop(&mut self) {
            unsafe { CoUninitialize() }
        }
    }
    struct Audio(IAudioClient);
    impl Drop for Audio {
        fn drop(&mut self) {
            let _ = unsafe { self.0.Stop() };
        }
    }
    struct Format(*mut WAVEFORMATEX);
    impl Drop for Format {
        fn drop(&mut self) {
            unsafe { CoTaskMemFree(Some(self.0.cast())) }
        }
    }
    let mut stage = "CoInitializeEx";
    let result = (|| -> windows::core::Result<()> {
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
            let _com = Com;
            stage = "CoCreateInstance(MMDeviceEnumerator)";
            let devices: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            stage = "GetDefaultAudioEndpoint(eRender/eConsole)";
            let device = devices.GetDefaultAudioEndpoint(eRender, eConsole)?;
            stage = "IMMDevice::Activate(IAudioClient)";
            let client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;
            stage = "GetMixFormat";
            let format = Format(client.GetMixFormat()?);
            let wave = *format.0;
            let mut tag = wave.wFormatTag;
            if tag == 0xfffe && wave.cbSize >= 22 {
                let subtype = (*format.0.cast::<WAVEFORMATEXTENSIBLE>()).SubFormat;
                // Only standard PCM/IEEE float extensible formats are accepted.
                if subtype == windows::core::GUID::from_u128(0x00000001_0000_0010_8000_00aa00389b71)
                {
                    tag = 1;
                }
                if subtype == windows::core::GUID::from_u128(0x00000003_0000_0010_8000_00aa00389b71)
                {
                    tag = 3;
                }
            }
            let (channels, rate, bits, stride) = (
                wave.nChannels,
                wave.nSamplesPerSec,
                wave.wBitsPerSample,
                wave.nBlockAlign,
            );
            trace.note(
                "audio.mix_format",
                format!("tag={tag} channels={channels} rate={rate} bits={bits} stride={stride}"),
            );
            stage = "validate mix format";
            if channels == 0
                || channels > 32
                || !(8000..=384000).contains(&rate)
                || !matches!((tag, bits), (3, 32) | (1, 16) | (1, 24) | (1, 32))
                || u32::from(stride) != u32::from(channels) * u32::from(bits / 8)
            {
                return Err(windows::core::Error::new(
                    windows::core::HRESULT(0x80004005u32 as i32),
                    "Unsupported system audio mix format",
                ));
            }
            stage = "IAudioClient::Initialize(LOOPBACK)";
            client.Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                AUDCLNT_STREAMFLAGS_LOOPBACK,
                1_000_000,
                0,
                format.0,
                None,
            )?;
            stage = "GetService(IAudioCaptureClient)";
            let reader: IAudioCaptureClient = client.GetService()?;
            drop(format);
            let _audio = Audio(client.clone());
            let _ = ready.send((rate, channels));
            stage = "accept encoder connection";
            let mut stream = loop {
                if stop.load(Ordering::Acquire) {
                    return Ok(());
                }
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(5))
                    }
                    Err(e) => {
                        return Err(windows::core::Error::new(
                            windows::core::HRESULT(0x80004005u32 as i32),
                            e.to_string(),
                        ));
                    }
                }
            };
            trace.note("audio.connected", "encoder connected to PCM stream");
            let io_error = |e: std::io::Error| {
                windows::core::Error::new(
                    windows::core::HRESULT(0x80004005u32 as i32),
                    e.to_string(),
                )
            };
            stage = "configure PCM socket";
            // Windows accepts can inherit the nonblocking listener mode. Make
            // it explicit and retain unsent bytes instead of failing write_all.
            stream.set_nonblocking(true).map_err(io_error)?;
            *socket.lock().unwrap() = Some(stream.try_clone().map_err(io_error)?);
            let mut frequency = 0;
            let mut counter = 0;
            stage = "QueryPerformanceFrequency";
            QueryPerformanceFrequency(&mut frequency)?;
            stage = "QueryPerformanceCounter";
            QueryPerformanceCounter(&mut counter)?;
            let origin = counter as f64 * 10_000_000.0 / frequency as f64;
            let started = Instant::now();
            stage = "IAudioClient::Start";
            client.Start()?;
            trace.note("audio.started", "WASAPI loopback started");
            let mut samples = VecDeque::<f32>::new();
            let mut pending = super::pcm_stream::PendingPcm::default();
            let mut written = 0u64;
            while !stop.load(Ordering::Acquire) {
                stage = "IAudioCaptureClient::GetNextPacketSize";
                while reader.GetNextPacketSize()? > 0 {
                    let (mut data, mut frames, mut flags, mut qpc) =
                        (std::ptr::null_mut(), 0, 0, 0);
                    stage = "IAudioCaptureClient::GetBuffer";
                    reader.GetBuffer(&mut data, &mut frames, &mut flags, None, Some(&mut qpc))?;
                    // Packet QPC values use 100 ns units. Keep a small playout delay so
                    // silence generation cannot consume a packet before it arrives.
                    let packet_start = if flags & 4 == 0 {
                        (((qpc as f64 - origin) * rate as f64 / 10_000_000.0).round() as i64).max(0)
                            as u64
                    } else {
                        written + (samples.len() / channels as usize) as u64
                    };
                    let end = written + (samples.len() / channels as usize) as u64;
                    if packet_start <= written + u64::from(rate) * 2 {
                        let gap = packet_start.saturating_sub(end) as usize * channels as usize;
                        samples.extend(std::iter::repeat_n(0.0, gap));
                        let skip = end.saturating_sub(packet_start).min(u64::from(frames)) as usize;
                        let silent =
                            flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0 || data.is_null();
                        for index in skip * channels as usize..frames as usize * channels as usize {
                            let value = if silent {
                                0.0
                            } else {
                                let p = data.add(index * (bits / 8) as usize);
                                match (tag, bits) {
                                    (3, 32) => f32::from_le_bytes(
                                        std::slice::from_raw_parts(p, 4).try_into().unwrap(),
                                    ),
                                    (1, 16) => i16::from_le_bytes([*p, *p.add(1)]) as f32 / 32768.0,
                                    (1, 24) => {
                                        i32::from_le_bytes([0, *p, *p.add(1), *p.add(2)]) as f32
                                            / 2147483648.0
                                    }
                                    _ => {
                                        i32::from_le_bytes(
                                            std::slice::from_raw_parts(p, 4).try_into().unwrap(),
                                        ) as f32
                                            / 2147483648.0
                                    }
                                }
                            };
                            samples.push_back(if value.is_finite() {
                                value.clamp(-1.0, 1.0)
                            } else {
                                0.0
                            });
                        }
                    }
                    stage = "IAudioCaptureClient::ReleaseBuffer";
                    reader.ReleaseBuffer(frames)?;
                    if samples.len() > rate as usize * channels as usize * 3 {
                        stage = "PCM buffer capacity";
                        return Err(windows::core::Error::new(
                            windows::core::HRESULT(0x80004005u32 as i32),
                            "System audio buffer overflow",
                        ));
                    }
                    stage = "IAudioCaptureClient::GetNextPacketSize";
                }
                let due = (started.elapsed().as_secs_f64() - 0.1).max(0.0) * rate as f64;
                let frames = (due as u64)
                    .saturating_sub(written)
                    .min(u64::from(rate) / 50) as usize;
                if frames > 0 {
                    let mut bytes = Vec::with_capacity(frames * channels as usize * 4);
                    for _ in 0..frames * channels as usize {
                        bytes.extend_from_slice(&samples.pop_front().unwrap_or(0.0).to_le_bytes());
                    }
                    stage = "write PCM to encoder";
                    pending.push(&bytes).map_err(io_error)?;
                    written += frames as u64;
                }
                stage = "send pending PCM to encoder";
                pending.pump(&mut stream).map_err(io_error)?;
                std::thread::sleep(Duration::from_millis(3));
            }
            let remaining = (started.elapsed().as_secs_f64() * rate as f64) as u64;
            let frames = remaining.saturating_sub(written).min(u64::from(rate) / 2) as usize;
            let mut bytes = Vec::with_capacity(frames * channels as usize * 4);
            for _ in 0..frames * channels as usize {
                bytes.extend_from_slice(&samples.pop_front().unwrap_or(0.0).to_le_bytes());
            }
            stage = "flush remaining PCM to encoder";
            pending.push(&bytes).map_err(io_error)?;
            let drain_deadline = Instant::now() + Duration::from_secs(2);
            while !pending.is_empty() {
                pending.pump(&mut stream).map_err(io_error)?;
                if Instant::now() >= drain_deadline {
                    return Err(io_error(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "PCM finalization exceeded 2 seconds",
                    )));
                }
                if !pending.is_empty() {
                    std::thread::sleep(Duration::from_millis(3));
                }
            }
            Ok(())
        }
    })();
    result.map_err(|e| format!("{stage}: {e}"))
}
