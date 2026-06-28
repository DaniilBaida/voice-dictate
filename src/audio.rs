// cpal::Stream is !Send on Windows (WASAPI COM objects are thread-affine).
// RecorderHandle owns a channel to a dedicated audio thread, making it Send.

use std::sync::mpsc;

enum Cmd {
    Start,
    Stop { reply: mpsc::SyncSender<anyhow::Result<Vec<u8>>> },
}

pub struct RecorderHandle {
    tx: mpsc::Sender<Cmd>,
}

impl RecorderHandle {
    pub fn start(&self) -> anyhow::Result<()> {
        self.tx.send(Cmd::Start).map_err(|_| anyhow::anyhow!("audio thread died"))
    }

    pub fn stop(&self) -> anyhow::Result<Vec<u8>> {
        let (reply_tx, reply_rx) = mpsc::sync_channel(1);
        self.tx
            .send(Cmd::Stop { reply: reply_tx })
            .map_err(|_| anyhow::anyhow!("audio thread died"))?;
        reply_rx.recv().map_err(|_| anyhow::anyhow!("audio thread died"))?
    }
}

pub fn spawn_audio_thread(samplerate: u32) -> anyhow::Result<RecorderHandle> {
    // Validate device exists at startup so we fail fast.
    {
        use cpal::traits::HostTrait;
        cpal::default_host()
            .default_input_device()
            .ok_or_else(|| anyhow::anyhow!("no microphone found"))?;
    }

    let (tx, rx) = mpsc::channel::<Cmd>();

    std::thread::Builder::new()
        .name("audio".into())
        .spawn(move || run_audio_thread(rx, samplerate))?;

    Ok(RecorderHandle { tx })
}

// ── Audio thread: cpal never leaves this thread ───────────────────────────────

fn run_audio_thread(rx: mpsc::Receiver<Cmd>, samplerate: u32) {
    let mut recorder = match Recorder::new(samplerate) {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("recorder init: {e}");
            return;
        }
    };
    for cmd in rx {
        match cmd {
            Cmd::Start => {
                if let Err(e) = recorder.start() {
                    tracing::error!("recorder start: {e}");
                }
            }
            Cmd::Stop { reply } => {
                let _ = reply.send(recorder.stop());
            }
        }
    }
}

struct RecorderInner {
    stream: cpal::Stream,
    frames: std::sync::Arc<std::sync::Mutex<Vec<f32>>>,
    sample_rate: u32,
    channels: u16,
}

struct Recorder {
    inner: Option<RecorderInner>,
    sample_rate: u32,
    channels: u16,
}

impl Recorder {
    fn new(samplerate: u32) -> anyhow::Result<Self> {
        use cpal::traits::{DeviceTrait, HostTrait};

        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow::anyhow!("no input device"))?;

        let default_config = device.default_input_config()?;
        let rate = if samplerate > 0 { samplerate } else { default_config.sample_rate().0 };
        let channels = default_config.channels();

        tracing::debug!(
            "audio: {}Hz {} ch device={}",
            rate,
            channels,
            device.name().unwrap_or_default()
        );

        Ok(Self { inner: None, sample_rate: rate, channels })
    }

    fn start(&mut self) -> anyhow::Result<()> {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

        if self.inner.is_some() {
            return Ok(());
        }

        let device = cpal::default_host()
            .default_input_device()
            .ok_or_else(|| anyhow::anyhow!("no input device"))?;

        let config = cpal::StreamConfig {
            channels: self.channels,
            sample_rate: cpal::SampleRate(self.sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };

        let frames = std::sync::Arc::new(std::sync::Mutex::new(Vec::<f32>::new()));
        let frames2 = std::sync::Arc::clone(&frames);

        let stream = device.build_input_stream(
            &config,
            move |data: &[f32], _| frames2.lock().unwrap().extend_from_slice(data),
            |e| tracing::error!("audio stream error: {e}"),
            None,
        )?;
        stream.play()?;

        self.inner = Some(RecorderInner {
            stream,
            frames,
            sample_rate: self.sample_rate,
            channels: self.channels,
        });
        Ok(())
    }

    fn stop(&mut self) -> anyhow::Result<Vec<u8>> {
        let inner = match self.inner.take() {
            Some(i) => i,
            None => return Ok(Vec::new()),
        };

        drop(inner.stream); // dropping stops the stream

        let frames = inner.frames.lock().unwrap().clone();
        if frames.is_empty() {
            return Ok(Vec::new());
        }

        // Mix down to mono and convert f32 -> i16 PCM
        let mono: Vec<i16> = frames
            .chunks(inner.channels as usize)
            .map(|ch| {
                let avg = ch.iter().sum::<f32>() / ch.len() as f32;
                (avg.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
            })
            .collect();

        encode_wav(&mono, inner.sample_rate)
    }
}

fn encode_wav(samples: &[i16], sample_rate: u32) -> anyhow::Result<Vec<u8>> {
    let mut buf = std::io::Cursor::new(Vec::new());
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::new(&mut buf, spec)?;
    for &s in samples {
        writer.write_sample(s)?;
    }
    writer.finalize()?;
    Ok(buf.into_inner())
}
