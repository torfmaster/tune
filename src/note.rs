use crate::math;
use crate::pitch::ConcertPitch;
use crate::pitch::Pitch;
use crate::ratio::Ratio;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;

pub const A5_NOTE: Note = Note { midi_number: 69 };

#[derive(Copy, Clone, Debug)]
pub struct Note {
    midi_number: i32,
}

impl Note {
    pub fn from_midi_number(midi_number: i32) -> Self {
        Self { midi_number }
    }

    pub fn midi_number(self) -> i32 {
        self.midi_number
    }

    pub fn steps_to(self, other: Note) -> i32 {
        other.midi_number - self.midi_number
    }

    pub fn pitch(self, concert_pitch: ConcertPitch) -> Pitch {
        Pitch::from_hz(
            concert_pitch.a5_hz()
                * Ratio::from_semitones(f64::from(self.midi_number - A5_NOTE.midi_number()))
                    .as_float(),
        )
    }
}

impl Display for Note {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let (octave, semitone) = math::div_mod_i32(self.midi_number, 12);

        let note_name = match semitone {
            0 => "C",
            1 => "C#/Db",
            2 => "D",
            3 => "D#/Eb",
            4 => "E",
            5 => "F",
            6 => "F#/Gb",
            7 => "G",
            8 => "G#/Ab",
            9 => "A",
            10 => "A#/Bb",
            11 => "B",
            other => unreachable!("value was {}", other),
        };

        let width = f.width().unwrap_or(0);
        write!(f, "{:width$} {}", note_name, octave, width = width)
    }
}