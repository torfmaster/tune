use crate::{effects::Delay, fluid::FluidSynth, synth::WaveformSynth};
use chrono::Local;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BufferSize, SampleRate, Stream, StreamConfig,
};
use hound::{SampleFormat, WavSpec, WavWriter};
use std::{
    fs::File, hash::Hash, io::BufWriter, sync::mpsc, sync::mpsc::Receiver, sync::mpsc::Sender,
};

const SAMPLE_RATE: u32 = 44100;

pub struct AudioModel {
    // This code isn't dead, actually. Audio is processed as long as the stream is alive.
    #[allow(dead_code)]
    stream: Stream,
    events: Sender<AudioEvent>,
}

enum AudioEvent {
    StartRecording,
    StopRecording,
}

struct AudioRenderer<E> {
    waveform_synth: WaveformSynth<E>,
    fluid_synth: FluidSynth,
    sample_rate: u32,
    delay: Delay,
    current_recording: Option<WavWriter<BufWriter<File>>>,
    events: Receiver<AudioEvent>,
}

impl AudioModel {
    pub fn new<E: 'static + Eq + Hash + Send>(
        waveform_synth: WaveformSynth<E>,
        fluid_synth: FluidSynth,
        buffer_size: u32,
        delay_secs: f32,
        delay_feedback: f32,
        delay_feedback_rotation_radians: f32,
    ) -> Self {
        let host = cpal::default_host();

        let device = host
            .default_output_device()
            .expect("failed to find a default output device");

        let (send, recv) = mpsc::channel();

        let mut renderer = AudioRenderer {
            waveform_synth,
            fluid_synth,
            sample_rate: SAMPLE_RATE,
            delay: Delay::new(
                (delay_secs * SAMPLE_RATE as f32).round() as usize,
                delay_feedback,
                delay_feedback_rotation_radians,
            ),
            current_recording: None,
            events: recv,
        };

        let stream_config = StreamConfig {
            channels: 2,
            buffer_size: BufferSize::Fixed(buffer_size),
            sample_rate: SampleRate(SAMPLE_RATE),
        };

        let stream = device
            .build_output_stream(
                &stream_config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    render_audio(&mut renderer, data)
                },
                |err| eprintln!("an error occurred on stream: {}", err),
            )
            .unwrap();

        Self {
            stream,
            events: send,
        }
    }

    pub fn start_recording(&self) {
        self.events.send(AudioEvent::StartRecording).unwrap();
    }

    pub fn stop_recording(&self) {
        self.events.send(AudioEvent::StopRecording).unwrap();
    }
}

fn render_audio<E: Eq + Hash>(renderer: &mut AudioRenderer<E>, buffer: &mut [f32]) {
    for sample in buffer.iter_mut() {
        *sample = 0.0;
    }
    renderer.fluid_synth.write(buffer);
    renderer.waveform_synth.write(buffer, renderer.sample_rate);
    renderer.delay.process(&mut buffer[..]);
    record_audio(renderer, buffer);
}

fn record_audio<E>(renderer: &mut AudioRenderer<E>, buffer: &mut [f32]) {
    for event in renderer.events.try_iter() {
        match event {
            AudioEvent::StartRecording => {
                renderer.current_recording = Some(create_writer());
                renderer.delay.mute()
            }
            AudioEvent::StopRecording => renderer.current_recording = None,
        }
    }
    if let Some(wav_writer) = &mut renderer.current_recording {
        for &sample in &buffer[..] {
            wav_writer.write_sample(sample).unwrap();
        }
    }
}

fn create_writer() -> WavWriter<BufWriter<File>> {
    let output_file_name = format!("microwave_{}.wav", Local::now().format("%Y%m%d_%H%M%S"));
    let spec = WavSpec {
        channels: 2,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };

    println!("[INFO] Created `{}`", output_file_name);
    WavWriter::create(output_file_name, spec).unwrap()
}
