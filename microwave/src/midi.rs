use crate::piano::PianoEngine;
use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use std::io::Write;
use std::sync::Arc;
use tune::{
    key_map::KeyMap,
    mts::{Channels, ScaleOctaveTuning, ScaleOctaveTuningMessage},
    note::Note,
    pitch::Pitched,
    ratio::Ratio,
    scale,
    temperament::{EqualTemperament, TemperamentPreference},
    tuning::Tuning,
};

pub fn print_midi_devices() {
    let midi_input = MidiInput::new("microwave").unwrap();
    println!("Available MIDI input devices:");
    for (index, port) in midi_input.ports().iter().enumerate() {
        let port_name = midi_input.port_name(port).unwrap();
        println!("({}) {}", index, port_name);
    }
}

pub fn connect_to_midi_device(
    target_device: usize,
    mut engine: Arc<PianoEngine>,
    midi_channel: u8,
    midi_logging: bool,
) -> MidiInputConnection<()> {
    let midi_input = MidiInput::new("microwave").unwrap();
    let port = &midi_input.ports()[target_device];

    midi_input
        .connect(
            &port,
            "microwave-input-connection",
            move |_, message, _| {
                process_midi_event(message, &mut engine, midi_channel, midi_logging)
            },
            (),
        )
        .unwrap()
}

fn process_midi_event(
    message: &[u8],
    engine: &mut Arc<PianoEngine>,
    input_channel: u8,
    midi_logging: bool,
) {
    if let Some(channel_message) = ChannelMessage::from_raw_message(message) {
        let stderr = std::io::stderr();
        let mut stderr = stderr.lock();
        if midi_logging {
            writeln!(stderr, "[DEBUG] MIDI message received:").unwrap();
            writeln!(stderr, "{:#?}", channel_message).unwrap();
            writeln!(stderr,).unwrap();
        }
        if channel_message.channel == input_channel {
            engine.handle_midi_event(channel_message.message_type);
        }
    } else {
        let stderr = std::io::stderr();
        let mut stderr = stderr.lock();
        writeln!(stderr, "[WARNING] Unsupported MIDI message received:").unwrap();
        for i in message {
            writeln!(stderr, "{:08b}", i).unwrap();
        }
        writeln!(stderr).unwrap();
    }
}

// https://www.midi.org/specifications-old/item/table-1-summary-of-midi-message

#[derive(Clone, Debug)]
pub struct ChannelMessage {
    pub channel: u8,
    pub message_type: ChannelMessageType,
}

impl ChannelMessage {
    pub fn from_raw_message(message: &[u8]) -> Option<ChannelMessage> {
        let status_byte = *message.get(0)?;
        let channel = status_byte & 0b0000_1111;
        let action = status_byte >> 4;
        let message_type = match action {
            0b1000 => ChannelMessageType::NoteOff {
                key: *message.get(1)?,
                velocity: *message.get(2)?,
            },
            0b1001 => ChannelMessageType::NoteOn {
                key: *message.get(1)?,
                velocity: *message.get(2)?,
            },
            0b1010 => ChannelMessageType::PolyphonicKeyPressure {
                key: *message.get(1)?,
                pressure: *message.get(2)?,
            },
            0b1011 => ChannelMessageType::ControlChange {
                controller: *message.get(1)?,
                value: *message.get(2)?,
            },
            0b1100 => ChannelMessageType::ProgramChange {
                program: *message.get(1)?,
            },
            0b1101 => ChannelMessageType::ChannelPressure {
                pressure: *message.get(1)?,
            },
            0b1110 => ChannelMessageType::PitchBendChange {
                value: u32::from(*message.get(1)?) + u32::from(*message.get(2)?) * 128,
            },
            _ => return None,
        };
        Some(ChannelMessage {
            channel,
            message_type,
        })
    }
}

#[derive(Copy, Clone, Debug)]
pub enum ChannelMessageType {
    NoteOff { key: u8, velocity: u8 },
    NoteOn { key: u8, velocity: u8 },
    PolyphonicKeyPressure { key: u8, pressure: u8 },
    ControlChange { controller: u8, value: u8 },
    ProgramChange { program: u8 },
    ChannelPressure { pressure: u8 },
    PitchBendChange { value: u32 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_note_off() {
        let message = ChannelMessage::from_raw_message(&[0b1000_0111, 88, 99]).unwrap();
        assert!(matches!(
            message,
            ChannelMessage {
                channel: 7,
                message_type: ChannelMessageType::NoteOff {
                    key: 88,
                    velocity: 99
                }
            }
        ));
    }

    #[test]
    fn parse_note_on() {
        let message = ChannelMessage::from_raw_message(&[0b1001_1000, 77, 88]).unwrap();
        assert!(matches!(
            message,
            ChannelMessage {
                channel: 8,
                message_type: ChannelMessageType::NoteOn {
                    key: 77,
                    velocity: 88
                }
            }
        ));
    }

    #[test]
    fn parse_polyphonic_key_pressure() {
        let message = ChannelMessage::from_raw_message(&[0b1010_1001, 66, 77]).unwrap();
        assert!(matches!(
            message,
            ChannelMessage {
                channel: 9,
                message_type: ChannelMessageType::PolyphonicKeyPressure {
                    key: 66,
                    pressure: 77
                }
            }
        ));
    }

    #[test]
    fn parse_control_change() {
        let message = ChannelMessage::from_raw_message(&[0b1011_1010, 55, 66]).unwrap();
        assert!(matches!(
            message,
            ChannelMessage {
                channel: 10,
                message_type: ChannelMessageType::ControlChange {
                    controller: 55,
                    value: 66
                }
            }
        ));
    }

    #[test]
    fn parse_program_change() {
        let message = ChannelMessage::from_raw_message(&[0b1100_1011, 44]).unwrap();
        assert!(matches!(
            message,
            ChannelMessage {
                channel: 11,
                message_type: ChannelMessageType::ProgramChange { program: 44 }
            }
        ));
    }

    #[test]
    fn parse_channel_pressure() {
        let message = ChannelMessage::from_raw_message(&[0b1101_1100, 33]).unwrap();
        assert!(matches!(
            message,
            ChannelMessage {
                channel: 12,
                message_type: ChannelMessageType::ChannelPressure { pressure: 33 }
            }
        ));
    }

    #[test]
    fn parse_pitch_bend_change() {
        let message = ChannelMessage::from_raw_message(&[0b1110_1101, 22, 33]).unwrap();
        assert!(matches!(
            message,
            ChannelMessage {
                channel: 13,
                message_type: ChannelMessageType::PitchBendChange { value: 4246 }
            }
        ));
    }
}

pub fn connect_to(device_name: &str) -> Option<MidiOutputConnection> {
    let midi_output = MidiOutput::new("microwave").unwrap();

    for port in midi_output.ports() {
        let port_name = midi_output.port_name(&port).unwrap();
        println!("{}", port_name);
        if port_name.contains(&device_name) {
            return Some(midi_output.connect(&port, "out_connection").unwrap());
        }
    }

    None
}

pub fn connect_to_in_port<T: Send, F: FnMut(u64, &[u8], &mut T) + Send + 'static>(
    device_name: &str,
    callback: F,
    data: T,
) -> Option<MidiInputConnection<T>> {
    let midi_input = MidiInput::new("microwave").unwrap();

    for port in midi_input.ports() {
        let port_name = midi_input.port_name(&port).unwrap();
        println!("{}", port_name);
        if port_name.contains(&device_name) {
            return Some(
                midi_input
                    .connect(&port, "in_connection", callback, data)
                    .unwrap(),
            );
        }
    }

    None
}

#[test]
fn octave_tuning() {
    connect_to("FLUID")
        .unwrap()
        .send(octave_scale_retune(22).sysex_bytes())
        .unwrap();
}

// First claviature
pub fn octave_scale_retune(num_divisions_per_octave: u16) -> ScaleOctaveTuningMessage {
    let root_note = Note::from_midi_number(60);
    let scale = scale::create_equal_temperament_scale(
        None,
        Ratio::from_octaves(1.0 / f64::from(num_divisions_per_octave)),
    );
    let key_map = KeyMap::root_at(root_note);

    let temperament = EqualTemperament::find()
        .with_preference(TemperamentPreference::Meantone)
        .by_edo(num_divisions_per_octave);

    let mut scale_octave_tuning = ScaleOctaveTuning::default();

    for i in 0..12 {
        let note_to_retune = root_note.plus_semitones(i);
        let original_pitch = note_to_retune.pitch();

        let num_primary_steps = i / 2;
        let num_secondary_steps = i % 2;
        let mapped_key = root_note
            .as_piano_key()
            .plus_steps(num_primary_steps * i32::from(temperament.primary_step()))
            .plus_steps(num_secondary_steps * i32::from(temperament.secondary_step()));
        let target_pitch = scale.with_key_map(&key_map).pitch_of(mapped_key);

        let detune = Ratio::between_pitches(original_pitch, target_pitch);

        *scale_octave_tuning.as_mut(note_to_retune.letter_and_octave().0) = detune;
    }

    // Minimize tuning, s.t. F# is NOT detuned
    let baseline = scale_octave_tuning.d.inv();
    for i in 0..12 {
        let letter = Note::from_midi_number(i).letter_and_octave().0;
        let detuning = scale_octave_tuning.as_mut(letter);
        *detuning = detuning.stretched_by(baseline);
    }

    ScaleOctaveTuningMessage::from_scale_octave_tuning(
        &scale_octave_tuning,
        Channels::All,
        Default::default(),
    )
    .unwrap()
}

#[test]
fn play_some_midi_notes() {
    let mut connection = connect_to("FLUID").unwrap();

    loop {
        connection.send(&note_on(0, 60, 100)).unwrap();
        std::thread::sleep_ms(1000);
        connection.send(&note_on(0, 85, 100)).unwrap();
        std::thread::sleep_ms(1000);
    }
}

pub fn note_off(channel: u8, note: u8, velocity: u8) -> [u8; 3] {
    [channel_msg(0b1000, channel), note, velocity]
}

pub fn note_on(channel: u8, note: u8, velocity: u8) -> [u8; 3] {
    [channel_msg(0b1001, channel), note, velocity]
}

fn channel_msg(prefix: u8, channel_nr: u8) -> u8 {
    prefix << 4 | channel_nr
}
