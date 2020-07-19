use crate::{
    model::{PianoEngineModel, SynthMode},
    Model,
};
use geom::Range;
use nannou::prelude::*;
use tune::{
    key::PianoKey, key_map::KeyMap, note::NoteLetter, ratio::Ratio, scale::Scale, tuning::Tuning,
};

pub fn view(app: &App, app_model: &Model, frame: Frame) {
    let engine_model = &app_model.engine_snapshot;

    let draw: Draw = app.draw();

    draw.background().color(DIMGRAY);

    let window_rect = app.window_rect();
    let (w, h) = window_rect.w_h();

    let note_at_left_border = (app_model.lowest_note.as_hz() / 440.0).log2() * 12.0;
    let note_at_right_border = (app_model.highest_note.as_hz() / 440.0).log2() * 12.0;

    let lowest_note_to_draw = note_at_left_border.floor();
    let highest_note_to_draw = note_at_right_border.ceil();

    let geometric_number_of_visible_notes = note_at_right_border - note_at_left_border;
    let lowest_key_position =
        (lowest_note_to_draw - note_at_left_border) / geometric_number_of_visible_notes;
    let key_stride = 1.0 / geometric_number_of_visible_notes;
    let key_width = key_stride * 0.9;

    if let Some(scale) = &engine_model.scale {
        let key_map = KeyMap::root_at(engine_model.root_note);
        let scale_with_key_map = scale.with_key_map(&key_map);

        let lowest_key: PianoKey = scale_with_key_map
            .find_by_pitch(app_model.lowest_note)
            .approx_value;

        let highest_key: PianoKey = scale_with_key_map
            .find_by_pitch(app_model.highest_note)
            .approx_value;

        for midi_number in lowest_key.midi_number()..=highest_key.midi_number() {
            let pitch = scale_with_key_map.pitch_of(PianoKey::from_midi_number(midi_number));
            let normalized_position = Ratio::between_pitches(app_model.lowest_note, pitch)
                .as_octaves()
                / Ratio::between_pitches(app_model.lowest_note, app_model.highest_note)
                    .as_octaves();

            let screen_position = (normalized_position as f32 - 0.5) * w;

            let line_color = if matches!(engine_model.synth_mode, SynthMode::Waveform)
                || (0..128).contains(&midi_number)
            {
                GRAY
            } else {
                INDIANRED
            };

            draw.line()
                .start(Point2 {
                    x: screen_position,
                    y: -app.window_rect().h() / 2.0,
                })
                .end(Point2 {
                    x: screen_position,
                    y: app.window_rect().h() / 2.0,
                })
                .color(line_color)
                .weight(2.0);
        }
    }

    render_hud(app_model, &engine_model, &draw, window_rect);

    for (stride_index, key_number) in
        (lowest_note_to_draw as i32..=highest_note_to_draw as i32).enumerate()
    {
        let note_to_draw = NoteLetter::A.in_octave(4).plus_semitones(key_number);

        let key_color = if note_to_draw == engine_model.root_note {
            LIGHTSTEELBLUE
        } else {
            match note_to_draw.letter_and_octave().0 {
                NoteLetter::Csh
                | NoteLetter::Dsh
                | NoteLetter::Fsh
                | NoteLetter::Gsh
                | NoteLetter::Ash => BLACK,
                _ => LIGHTGRAY,
            }
        };

        let key_position = lowest_key_position + stride_index as f64 * key_stride;
        draw.rect()
            .color(key_color)
            .w(key_width as f32 * w)
            .h(h / 2.0)
            .x((key_position as f32 - 0.5) * w)
            .y(-h / 4.0);
    }

    let mut freqs_hz = engine_model
        .pressed_keys
        .iter()
        .map(|(_, pressed_key)| pressed_key.pitch)
        .collect::<Vec<_>>();
    freqs_hz.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mut curr_slice_window = freqs_hz.as_slice();

    while let Some((second, others)) = curr_slice_window.split_last() {
        let normalized_position = Ratio::between_pitches(app_model.lowest_note, *second)
            .as_octaves()
            / Ratio::between_pitches(app_model.lowest_note, app_model.highest_note).as_octaves();

        let screen_position = (normalized_position as f32 - 0.5) * w;

        draw.line()
            .start(Point2 {
                x: screen_position,
                y: -app.window_rect().h() / 2.0,
            })
            .end(Point2 {
                x: screen_position,
                y: app.window_rect().h() / 2.0,
            })
            .color(WHITE)
            .weight(2.0);

        let mut curr_rect = Rect {
            x: Range::new(screen_position, screen_position + 1000.0),
            y: Range::from_pos_and_len(0.0, 24.0),
        }
        .align_top_of(window_rect);

        draw.text(&format!("{:.0} Hz", second.as_hz()))
            .xy(curr_rect.xy())
            .wh(curr_rect.wh())
            .left_justify()
            .color(RED)
            .font_size(24);

        for first in others.iter() {
            let approximation = Ratio::between_pitches(*first, *second).nearest_fraction(11);

            let width = approximation.deviation.as_semitones() * key_stride * w as f64;
            let deviation_bar_rect = Rect {
                x: Range::new(screen_position - width as f32, screen_position),
                y: Range::from_pos_and_len(0.0, 24.0),
            }
            .below(curr_rect);

            draw.rect()
                .xy(deviation_bar_rect.xy())
                .wh(deviation_bar_rect.wh())
                .color(DEEPSKYBLUE);

            let deviation_text_rect = curr_rect.below(curr_rect);

            draw.text(&format!(
                "{}/{} [{:.0}c]",
                approximation.numer,
                approximation.denom,
                approximation.deviation.as_cents().abs()
            ))
            .xy(deviation_text_rect.xy())
            .wh(deviation_text_rect.wh())
            .left_justify()
            .color(BLACK)
            .font_size(24);

            curr_rect = deviation_text_rect;
        }
        curr_slice_window = others;
    }

    draw.to_frame(app, &frame).unwrap();
}

fn render_hud(app_model: &Model, engine_model: &PianoEngineModel, draw: &Draw, window_rect: Rect) {
    let hud_rect = Rect::from_w_h(window_rect.w(), 10.0 * 24.0)
        .bottom_left_of(window_rect)
        .shift_y(window_rect.h() / 2.0);

    let scale_text = engine_model
        .scale
        .as_ref()
        .map(Scale::description)
        .unwrap_or("Continuous");

    let waveform_text = match engine_model.synth_mode {
        SynthMode::OnlyWaveform | SynthMode::Waveform => format!(
            "Waveform: {} - {}",
            engine_model.waveform_number, engine_model.waveform_name,
        ),
        SynthMode::Fluid => format!(
            "Preset: {} - {}",
            app_model.selected_program.program_number,
            app_model
                .selected_program
                .program_name
                .as_deref()
                .unwrap_or("Unknown"),
        ),
    };

    let legato_text = if engine_model.legato { "ON" } else { "OFF" };

    let hud_text = format!(
        "Scale: {scale}\n\
         {waveform_text}\n\
         <up>/<down>/<space> to change\n\
         Root Note: {root_note}\n\
         <left>/<right> to change\n\
         Range: {from:.0}..{to:.0} Hz\n\
         <scroll> to change\n\
         Legato: {legato}\n\
         <Ctrl+L> to change",
        scale = scale_text,
        waveform_text = waveform_text,
        root_note = engine_model.root_note,
        from = app_model.lowest_note.as_hz(),
        to = app_model.highest_note.as_hz(),
        legato = legato_text,
    );

    draw.text(&hud_text)
        .xy(hud_rect.xy())
        .wh(hud_rect.wh())
        .left_justify()
        .color(LIGHTGREEN)
        .font_size(24);
}
