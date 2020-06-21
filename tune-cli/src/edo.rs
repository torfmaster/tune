use std::{convert::TryFrom, io};
use tune::{
    key::{Keyboard, PianoKey},
    math,
    ratio::Ratio,
    temperament::{EqualTemperament, TemperamentPreference, TemperamentType},
};

// enum Mode {}

// Locrian    (sLLsLLL)
// Phrygian   (sLLLsLL)
// Aeolian    (LsLLsLL)
// Dorian     (LsLLLsL)
// Mixolydian (LLsLLsL)
// Ionian     (LLsLLLs)
// Lydian     (LLLsLLs)

// MelodicMinor    (LsLLLLs)
// Phrygodorian    (sLLLLsL)
// LydianAugmented (LLLLsLs)
// LydianDominant  (LLLsLsL)
// MelodicMajor    (LLsLsLL)
// HalfDiminished  (LsLsLLLL)
// Altered         (sLsLLLL)

pub fn print_info(mut dst: impl io::Write, num_steps_per_octave: u16) -> io::Result<()> {
    let temperament = EqualTemperament::find().by_edo(num_steps_per_octave);
    print_temperament(&mut dst, &temperament)?;
    match temperament.temperament_type() {
        TemperamentType::Meantone => {
            if let Some(porcupine) = temperament.as_porcupine() {
                writeln!(dst)?;
                print_temperament(dst, &porcupine)?;
            }
        }
        TemperamentType::Porcupine => {}
    }

    Ok(())
}

pub fn print_temperament(
    mut dst: impl io::Write,
    temperament: &EqualTemperament,
) -> io::Result<()> {
    writeln!(
        dst,
        "---- Properties of {}-EDO ({}) ----",
        temperament.num_steps_per_octave(),
        temperament.temperament_type()
    )?;
    writeln!(dst)?;

    writeln!(dst, "Number of cycles: {}", temperament.num_cycles())?;
    writeln!(
        dst,
        "1 fifth = {} EDO steps = {:#} = Pythagorean {:#}",
        temperament.num_steps_per_fifth(),
        temperament.size_of_fifth(),
        temperament
            .size_of_fifth()
            .deviation_from(Ratio::from_float(1.5))
    )?;
    writeln!(
        dst,
        "1 primary step = {} EDO steps",
        temperament.primary_step()
    )?;
    writeln!(
        dst,
        "1 secondary step = {} EDO steps",
        temperament.secondary_step()
    )?;
    write!(dst, "1 sharp = {} EDO steps", temperament.sharpness())?;
    if temperament.sharpness() < 0 {
        writeln!(dst, " (Mavila)")?;
    } else {
        writeln!(dst)?;
    }
    writeln!(
        dst,
        "Dorian scale: {} {} {} {} {} {} {} {}",
        0,
        temperament.primary_step(),
        temperament.primary_step() + temperament.secondary_step(),
        2 * temperament.primary_step() + temperament.secondary_step(),
        3 * temperament.primary_step() + temperament.secondary_step(),
        4 * temperament.primary_step() + temperament.secondary_step(),
        4 * temperament.primary_step() + 2 * temperament.secondary_step(),
        5 * temperament.primary_step() + 2 * temperament.secondary_step()
    )?;
    writeln!(dst)?;

    writeln!(dst, "-- Keyboard layout --")?;
    let keyboard = Keyboard::root_at(PianoKey::from_midi_number(0))
        .with_steps_of(&temperament)
        .coprime();
    for y in (-5i16..5).rev() {
        for x in 0..10 {
            write!(
                dst,
                "{:^4}",
                keyboard
                    .get_key(x, y)
                    .midi_number()
                    .rem_euclid(i32::from(temperament.num_steps_per_octave())),
            )?;
        }
        writeln!(dst)?;
    }
    writeln!(dst)?;

    writeln!(dst, "-- Scale steps --")?;

    let location_of_minor_third = (Ratio::from_float(6.0 / 5.0).as_octaves()
        * f64::from(temperament.num_steps_per_octave()))
    .round() as u16;
    let location_of_major_third = (Ratio::from_float(5.0 / 4.0).as_octaves()
        * f64::from(temperament.num_steps_per_octave()))
    .round() as u16;
    let location_of_fourth = temperament.num_steps_per_octave() - temperament.num_steps_per_fifth();
    let location_of_fifth = temperament.num_steps_per_fifth();

    for index in 0..temperament.num_steps_per_octave() {
        write!(dst, "{:>3}. ", index,)?;
        write!(dst, "{}", temperament.get_heptatonic_name(i32::from(index)))?;
        if index == location_of_minor_third {
            write!(dst, " **JI m3rd**")?;
        }
        if index == location_of_major_third {
            write!(dst, " **JI M3rd**")?;
        }
        if index == location_of_fourth {
            write!(dst, " **JI P4th**")?;
        }
        if index == location_of_fifth {
            write!(dst, " **JI P5th**")?;
        }
        writeln!(dst)?;
    }

    Ok(())
}

pub fn print_claviatures(mut dst: impl io::Write, num_steps_per_octave: u16) -> io::Result<()> {
    let temperament = EqualTemperament::find()
        .with_second_best_fifth_allowed(false)
        .with_preference(TemperamentPreference::Meantone)
        .by_edo(num_steps_per_octave);

    assert_eq!(temperament.num_cycles(), 1);

    let mut num_claviatures = i16::try_from(num_steps_per_octave / 12).unwrap();
    if num_steps_per_octave % 12 != 0 {
        num_claviatures += 1;
    }

    for claviature_index in 0..num_claviatures {
        // TODO: Shift geschickter berechnen, vielleicht zentrieren?

        let seven_fifths = temperament.sharpness() - temperament.secondary_step();
        let offset = claviature_index * seven_fifths;
        for primary_step in 0..6 {
            let key = math::i16_rem_u16(
                primary_step * temperament.primary_step() + offset,
                num_steps_per_octave,
            );
            write!(dst, "{:^4}", key).unwrap()
        }
        writeln!(dst).unwrap();
        for primary_step in 0..6 {
            let key = math::i16_rem_u16(
                primary_step * temperament.primary_step() + temperament.secondary_step() + offset,
                num_steps_per_octave,
            );
            write!(dst, "{:^4}", key).unwrap()
        }
        writeln!(dst).unwrap();
        writeln!(dst).unwrap();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_name() {
        // TODO: 33 funktioniert nicht
        print_claviatures(io::stdout(), 23).unwrap();
    }
}
