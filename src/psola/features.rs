use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use crate::consts;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct PsolaFeatures {
    // for UTAU modulation
    pub base_f0: f64,
    // Actual PSOLA features
    pub f0: Vec<f64>,
    pub audio: Vec<f64>,
}

pub fn to_feature_path<P: AsRef<Path>>(path: P) -> PathBuf {
    // Converts any path to the feature file path
    let path = path.as_ref();
    let mut fname = path.file_stem().unwrap().to_owned();
    fname.push("_wav");
    path.with_file_name(fname)
        .with_extension(consts::FEATURE_EXT)
}

fn calculate_base_f0(f0: &Vec<f64>) -> f64 {
    // Get base F0. Averages the whole F0 curve with strong bias on flat areas.
    let n = f0.len();
    let mut base_f0 = 0.;
    let mut tally = 0.;

    for i in 0..n {
        if f0[i] >= consts::F0_FLOOR && f0[i] <= consts::F0_CEIL {
            let q = if i == 0 {
                f0[1] - f0[0]
            } else if i == n - 1 {
                f0[n - 2] - f0[n - 1]
            } else {
                0.5 * (f0[i + 1] - f0[i - 1])
            };

            let weight = (-q * q).exp2(); // Quicker bell curve
            base_f0 += f0[i] * weight;
            tally += weight;
        }
    }

    if tally > 0. {
        base_f0 /= tally;
    }
    base_f0
}

fn estimate_f0_from_audio(audio: &[f64], frame_period: f64, sample_rate: u32) -> Vec<f64> {
    // Simple zero-crossing based F0 estimation for PSOLA
    let frame_size = ((frame_period / 1000.0) * sample_rate as f64) as usize;
    let hop_size = frame_size;
    let mut f0 = Vec::new();
    
    let min_period = (sample_rate as f64 / consts::F0_CEIL) as usize;
    let max_period = (sample_rate as f64 / consts::F0_FLOOR) as usize;
    
    let mut pos = 0;
    while pos + max_period < audio.len() {
        let frame_start = pos;
        let mut zero_crossings = Vec::new();
        
        // Find zero crossings in the frame
        for i in frame_start..(frame_start + max_period).min(audio.len() - 1) {
            if audio[i] * audio[i + 1] < 0.0 {
                zero_crossings.push(i);
            }
        }
        
        // Estimate period from zero crossings
        let mut periods = Vec::new();
        for i in 1..zero_crossings.len() {
            let period = zero_crossings[i] - zero_crossings[i - 1];
            if period >= min_period && period <= max_period {
                periods.push(period);
            }
        }
        
        if !periods.is_empty() {
            let avg_period: usize = periods.iter().sum::<usize>() / periods.len();
            let estimated_f0 = sample_rate as f64 / (2.0 * avg_period as f64);
            f0.push(estimated_f0);
        } else {
            f0.push(0.0);
        }
        
        pos += hop_size;
    }
    
    // Pad or trim to match expected length
    let expected_frames = audio.len() / hop_size + 1;
    while f0.len() < expected_frames {
        f0.push(0.0);
    }
    f0.truncate(expected_frames);
    
    f0
}

pub fn generate_features<P: AsRef<Path>>(
    path: P,
    audio: Vec<f64>,
    _threshold: Option<f64>,
) -> Result<PsolaFeatures> {
    // Generate PSOLA features - store original audio and estimated F0
    let f0 = estimate_f0_from_audio(&audio, consts::FRAME_PERIOD, consts::SAMPLE_RATE);
    let base_f0 = calculate_base_f0(&f0);

    let features = PsolaFeatures {
        base_f0,
        f0,
        audio,
    };

    let feature_path = to_feature_path(path);
    let bin = bincode::serialize(&features)?;

    let mut feature_file = File::create(feature_path)?;
    feature_file.write_all(&bin)?;
    Ok(features)
}

pub fn read_features<P: AsRef<Path>>(path: P) -> Result<PsolaFeatures> {
    // Read PSOLA feature file
    let mut bin = Vec::new();
    let mut f = File::open(path)?;
    f.read_to_end(&mut bin)?;

    let features: PsolaFeatures = bincode::deserialize(&bin)?;
    Ok(features)
}

#[cfg(test)]
mod tests {
    use super::{generate_features, read_features};
    use crate::audio::read_write::{read_audio, write_audio};
    use crate::consts;
    use std::path::Path;
    use std::time::Instant;

    #[test]
    fn test_psola() {
        let path = Path::new("test/test.wav");
        let feature_path = path.with_extension(consts::FEATURE_EXT);
        let synth_path = path.with_extension("syn.wav");
        let audio = read_audio(path).expect("Cannot read audio");
        println!("gt: {}", audio.len());

        let now = Instant::now();
        let features =
            generate_features(&path, audio, None).expect("Cannot generate PSOLA features");
        println!("Feature Generation: {:.2?}", now.elapsed());
        let now = Instant::now();
        let features = read_features(&feature_path).expect("Cannot read PSOLA features");
        println!("Read features from file: {:.2?}", now.elapsed());

        let syn = crate::psola::synthesis::synthesize(&features.f0, &features.audio, consts::SAMPLE_RATE as f64, consts::FRAME_PERIOD);
        println!("synthesis: {}", syn.len());

        write_audio(synth_path, &syn).expect("Cannot write");
    }
}
