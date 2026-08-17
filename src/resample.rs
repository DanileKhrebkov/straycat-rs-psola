use crate::audio::post_process::{peak_compression, peak_normalization};
use crate::audio::read_write::{read_audio, write_audio};
use crate::flags::parser::Flags;
use crate::interpolator::interp::{self, Interpolator};
use crate::parser::ResamplerArgs;
use crate::psola::features::{generate_features, read_features, to_feature_path};
use crate::psola::synthesis::synthesize as psola_synthesize;
use crate::util::{self, smoothstep};
use crate::{consts, filter, pitchbend};
use anyhow::Result;
use biquad::{DirectForm2Transposed, Q_BUTTERWORTH_F64};
use std::path::Path;

pub fn run(args: ResamplerArgs) -> Result<()> {
    // Main resampler function using PSOLA
    let null_out = &args.out_file == "nul"; // null file from Initialize freq. map args
    let flags: Flags = args.flags.replace("/", "").parse()?; // parse flags

    // input file and feature file
    let in_file = Path::new(&args.in_file);
    let feature_path = to_feature_path(in_file);

    // force generate feature file if enabled
    if let Some(_threshold) = flags.generate_features {
        // PSOLA doesn't use D4C threshold, but keep the flag for compatibility
        println!("Forcing feature generation (PSOLA mode).");
        let audio = read_audio(&args.in_file)?;
        generate_features(&args.in_file, audio, None)?;
    }

    // generate feature file if it doesn't exist
    let features = if !feature_path.exists() {
        println!("Generating PSOLA features.");
        let audio = read_audio(&args.in_file)?;
        generate_features(&args.in_file, audio, None)?
    } else {
        println!("Reading PSOLA features.");
        read_features(&feature_path)?
    };

    // skip null output
    if null_out {
        println!("Null output file. Skipping.");
        return Ok(());
    }

    let out_file = Path::new(&args.out_file); // output file
    let velocity = (1. - args.velocity / 100.).exp2(); // velocity as stretch
    let volume = args.volume / 100.; // volume
    let modulation = args.modulation / 100.; // mod

    let feature_length = features.f0.len();
    let vuv: Vec<bool> = features.f0.iter().map(|f0| *f0 != 0.).collect();
    let f0_off: Vec<f64> = features
        .f0
        .iter()
        .map(|f0| {
            if *f0 == 0. {
                0.
            } else {
                12. * (f0.log2() - features.base_f0.log2())
            }
        })
        .collect();

    println!("Calculating timing.");
    let fps = 1000. / consts::FRAME_PERIOD; // PSOLA frames per second (WORLD compatible)
    let t_features: Vec<f64> = util::arange(feature_length as i32)
        .into_iter()
        .map(|x| x / fps)
        .collect();
    let feature_length_sec = feature_length as f64 / fps;
    let start = args.offset / 1000.;
    let end = args.cutoff / 1000.;
    let end = if end < 0. {
        start - end
    } else {
        feature_length_sec - end
    };
    let consonant = start + args.consonant / 1000.;

    println!("Preparing interpolation.");

    let t_consonant = util::linspace(
        start,
        consonant,
        (velocity * args.consonant / consts::FRAME_PERIOD) as usize,
        false,
    );

    let length_req = args.length / 1000.;
    let stretch_length = end - consonant;
    let t_stretch = if stretch_length > length_req {
        let con_idx = (consonant * fps) as usize;
        let len_idx = (length_req * fps) as usize;
        t_features[con_idx..(con_idx + len_idx).min(feature_length - 1)].to_vec()
    } else {
        util::linspace(consonant, end, (length_req * fps) as usize, true)
    };
    let consonant = velocity * args.consonant / 1000.; // timestamp of consonant in the render

    let t_render: Vec<f64> = t_consonant
        .into_iter()
        .chain(t_stretch.into_iter())
        .map(|x| x * fps)
        .collect();
    let render_length = t_render.len();

    println!("Interpolating F0 contour.");
    let f0_off_interp = interp::Akima::new(&f0_off);

    let f0_off_render = f0_off_interp.sample_with_vec(&t_render);
    let vuv_render: Vec<bool> = t_render
        .iter()
        .map(|i| vuv[(*i as usize).clamp(0, feature_length - 1)])
        .collect();
    let t_sec: Vec<f64> = util::arange(render_length as i32)
        .iter()
        .map(|x| x / fps)
        .collect();

    println!("Interpreting pitchbend.");
    println!("Checking flags.");
    if flags.pitch_offset != 0. {
        println!("Applying pitch offset.");
    }
    let pitch = pitchbend::parser::pitch_string_to_midi(args.pitchbend)?;
    let pps = 8. * args.tempo / 5.; // pitchbend points per second
    let pitch_interp = interp::Akima::new(&pitch);
    let t_pitch: Vec<f64> = t_sec.iter().map(|x| x * pps).collect();
    let pitch_render = pitch_interp.sample_with_vec(&t_pitch);

    let f0_render: Vec<f64> = pitch_render
        .iter()
        .zip(f0_off_render.into_iter().zip(vuv_render.iter()))
        .map(|(pitch, (f0_off, vuv))| {
            if *vuv {
                util::midi_to_hz(
                    *pitch + args.pitch as f64 + flags.pitch_offset / 100. + f0_off * modulation,
                )
            } else {
                0.
            }
        })
        .collect();

    // Apply fry to F0 contour
    if flags.fry_enable != 0. {
        println!("Applying fry.");
        apply_fry_f0(&mut f0_render.clone(), &vuv_render, &t_sec, consonant, &flags);
    }

    // PSOLA synthesis - directly synthesize from original audio with new F0
    println!("Synthesizing with PSOLA.");
    let sample_rate = consts::SAMPLE_RATE as f64;
    let mut syn = psola_synthesize(&f0_render, &features.audio, sample_rate, consts::FRAME_PERIOD);

    // Apply volume
    syn.iter_mut().for_each(|s| *s *= volume);

    // Note: PSOLA doesn't support these WORLD-specific features:
    // - gender/formant shifting (requires spectral manipulation)
    // - breathiness control (requires harmonic/aperiodic separation)
    // - aperiodic mix (requires aperiodicity analysis)
    // - growl (requires harmonic synthesis)
    
    // Print warnings for unsupported flags
    if flags.gender != 0. {
        println!("Warning: Gender flag not supported in PSOLA mode.");
    }
    if flags.breathiness != 50. {
        println!("Warning: Breathiness flag not supported in PSOLA mode.");
    }
    if flags.aperiodic_mix != 0. {
        println!("Warning: Aperiodic mix flag not supported in PSOLA mode.");
    }
    if flags.growl != 0. {
        println!("Warning: Growl flag not supported in PSOLA mode.");
    }

    if flags.tremolo != 0. {
        println!("Adding tremolo.");
        let mut pitch_raw = pitch_interp.sample_with_vec(&t_pitch);
        let t_syn: Vec<f64> = util::arange(syn.len() as i32)
            .iter()
            .map(|x| x / consts::SAMPLE_RATE as f64)
            .collect();
        tremolo(&mut syn, &mut pitch_raw, &t_syn, fps, flags.tremolo / 100.)?;
    }

    if flags.peak_compression != 0. {
        println!("Compressing render.");
        peak_compression(&mut syn, flags.peak_compression / 100.)?;
    }

    if flags.peak_normalization >= 0. {
        println!("Normalizing render.");
        peak_normalization(&mut syn, flags.peak_normalization);
    }

    write_audio(out_file, &syn)?;
    Ok(())
}

// Flag functions for PSOLA - simplified versions that work with F0 only
fn apply_fry_f0(
    f0: &mut Vec<f64>,
    vuv: &Vec<bool>,
    t: &Vec<f64>,
    consonant: f64,
    flags: &Flags,
) {
    // fake fry with a low pitchbend
    let fry_length = flags.fry_enable / 1000.;
    let fry_transition = 0.5 * flags.fry_transition.copysign(flags.fry_enable) / 1000.;
    let fry_offset = flags.fry_offset / 1000.;
    f0.iter_mut()
        .zip(t.iter().zip(vuv.iter()))
        .for_each(|(f0_val, (t, vuv))| {
            let t = t - consonant - fry_offset;
            let amt = smoothstep(
                -fry_length - fry_transition,
                -fry_length + fry_transition,
                t,
            ) * smoothstep(fry_transition, -fry_transition, t);
            if *vuv {
                *f0_val = util::lerp(*f0_val, flags.fry_pitch, amt);
            }
        });
}

fn tremolo(
    signal: &mut Vec<f64>,
    pitch: &Vec<f64>,
    t: &Vec<f64>,
    fps: f64,
    strength: f64,
) -> Result<()> {
    // Add tremolo to signal based on the pitchbend
    // double approximate derivative leads to approximate inverted vibrato cuz of how the derivative of trig functions work <3
    let tremolo: Vec<f64> = pitch.windows(2).map(|x| x[1] - x[0]).collect();
    let mut tremolo: Vec<f64> = tremolo.windows(2).map(|x| -40. * (x[1] - x[0])).collect(); // -40 is just a value to scale and invert

    // filter out vibratos out of range. vibrato range used is 4~8 Hz. this'll also remove some imprecision from the discrete diffs
    let tremolo_highpass =
        filter::make_coefficients(biquad::Type::HighPass, fps, 4., Q_BUTTERWORTH_F64)?;
    let tremolo_lowpass =
        filter::make_coefficients(biquad::Type::LowPass, fps, 8., Q_BUTTERWORTH_F64)?;
    let mut tremolo_highpass = DirectForm2Transposed::<f64>::new(tremolo_highpass);
    let mut tremolo_lowpass = DirectForm2Transposed::<f64>::new(tremolo_lowpass);

    filter::forward_backward_filter(&mut tremolo, &mut tremolo_highpass, 1);
    filter::forward_backward_filter(&mut tremolo, &mut tremolo_lowpass, 1);

    // interpolate to sampling rate
    let tremolo_interp = interp::Akima::new(&tremolo);
    let tremolo_signal = tremolo_interp.sample_with_vec(&t.iter().map(|x| x * fps - 2.).collect());

    // use the isolated vibratos as an envelope
    signal
        .iter_mut()
        .zip(tremolo_signal.iter())
        .for_each(|(x, env)| *x *= (std::f64::consts::LN_10 * env * strength / 5.).exp());

    Ok(())
}
