use crate::consts;
use crate::util;
use std::f64::consts::PI;

/// PSOLA (Pitch Synchronous Overlap and Add) synthesis
/// 
/// This function modifies the pitch of an audio signal using PSOLA technique.
/// 
/// # Arguments
/// * `f0_target` - Target F0 contour (in Hz)
/// * `audio` - Original audio signal
/// * `sample_rate` - Sample rate in Hz
/// * `frame_period` - Frame period in ms (typically 5ms for WORLD compatibility)
pub fn synthesize(
    f0_target: &[f64],
    audio: &[f64],
    sample_rate: f64,
    frame_period: f64,
) -> Vec<f64> {
    let fps = 1000.0 / frame_period; // frames per second
    let hop_original = (sample_rate / fps) as usize;
    
    // Calculate original F0 from audio if not provided or all zeros
    let f0_original = estimate_f0_simple(audio, sample_rate, hop_original);
    
    // Determine output length based on target F0 length
    let output_length = (f0_target.len() as f64 * hop_original as f64) as usize;
    let mut output = vec![0.0; output_length + hop_original * 2];
    let mut output_energy = vec![0.0; output_length + hop_original * 2];
    
    // Process each frame
    for (i, &target_f0) in f0_target.iter().enumerate() {
        let pos_original = i * hop_original;
        let pos_target = (i as f64 * hop_original as f64) as usize;
        
        if target_f0 > 0.0 && pos_original + hop_original < audio.len() {
            // Extract a window around this position from original audio
            let window_size = (sample_rate / target_f0 * 2.0) as usize;
            let window_size = window_size.max(hop_original).min(audio.len() - pos_original - 1);
            
            if window_size == 0 {
                continue;
            }
            
            // Apply pitch scaling factor
            let pitch_ratio = if f0_original[i] > 0.0 {
                target_f0 / f0_original[i]
            } else {
                1.0
            };
            
            // Extract and resample the segment
            let segment = extract_and_resample_segment(
                audio,
                pos_original,
                window_size,
                pitch_ratio,
                sample_rate,
            );
            
            // Apply Hanning window
            let windowed = apply_hanning_window(&segment);
            
            // Overlap-add to output
            for (j, &sample) in windowed.iter().enumerate() {
                let out_pos = pos_target + j;
                if out_pos < output.len() {
                    output[out_pos] += sample;
                    output_energy[out_pos] += 1.0;
                }
            }
        }
    }
    
    // Normalize by energy
    for (i, &energy) in output_energy.iter().enumerate() {
        if energy > 0.0 {
            output[i] /= energy;
        }
    }
    
    output
}

fn estimate_f0_simple(audio: &[f64], sample_rate: f64, hop_size: usize) -> Vec<f64> {
    // Simple autocorrelation-based F0 estimation
    let min_period = (sample_rate / consts::F0_CEIL) as usize;
    let max_period = (sample_rate / consts::F0_FLOOR) as usize;
    let mut f0 = Vec::new();
    
    let mut pos = 0;
    while pos + max_period < audio.len() {
        let frame_start = pos;
        let frame_end = (pos + max_period).min(audio.len());
        
        if frame_end - frame_start < max_period {
            f0.push(0.0);
            pos += hop_size;
            continue;
        }
        
        // Find best period using autocorrelation
        let mut best_period = 0;
        let mut best_correlation = 0.0;
        
        for period in min_period..=max_period.min(frame_end - frame_start - 1) {
            let mut correlation = 0.0;
            let mut energy = 0.0;
            
            for i in 0..(frame_end - frame_start - period) {
                correlation += audio[frame_start + i] * audio[frame_start + i + period];
                energy += audio[frame_start + i].powi(2);
            }
            
            if energy > 0.0 {
                correlation /= energy;
                if correlation > best_correlation {
                    best_correlation = correlation;
                    best_period = period;
                }
            }
        }
        
        if best_period > 0 {
            f0.push(sample_rate / best_period as f64);
        } else {
            f0.push(0.0);
        }
        
        pos += hop_size;
    }
    
    f0
}

fn extract_and_resample_segment(
    audio: &[f64],
    start: usize,
    length: usize,
    pitch_ratio: f64,
    _sample_rate: f64,
) -> Vec<f64> {
    // Simple resampling for pitch modification
    let new_length = (length as f64 / pitch_ratio) as usize;
    let mut segment = Vec::with_capacity(new_length);
    
    for i in 0..new_length {
        let src_pos = i as f64 * pitch_ratio;
        let src_idx = src_pos.floor() as usize;
        let frac = src_pos.fract();
        
        if src_idx + 1 < start + length && src_idx >= start {
            let sample = util::lerp(audio[start + src_idx], audio[start + src_idx + 1], frac);
            segment.push(sample);
        } else if src_idx >= start && src_idx < start + length {
            segment.push(audio[src_idx]);
        } else {
            segment.push(0.0);
        }
    }
    
    segment
}

fn apply_hanning_window(signal: &[f64]) -> Vec<f64> {
    let n = signal.len();
    if n == 0 {
        return Vec::new();
    }
    
    signal
        .iter()
        .enumerate()
        .map(|(i, &s)| {
            let window = 0.5 * (1.0 - (2.0 * PI * i as f64 / (n - 1) as f64).cos());
            s * window
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_psola_synthesis() {
        // Create a simple sine wave
        let sample_rate = 44100.0;
        let duration = 0.1; // 100ms
        let freq = 440.0; // A4
        let n_samples = (sample_rate * duration) as usize;
        
        let audio: Vec<f64> = (0..n_samples)
            .map(|i| (2.0 * PI * freq * i as f64 / sample_rate).sin())
            .collect();
        
        // Create target F0 (same as original)
        let frame_period = 5.0;
        let fps = 1000.0 / frame_period;
        let n_frames = (duration * 1000.0 / frame_period) as usize;
        let f0_target = vec![freq; n_frames];
        
        let output = synthesize(&f0_target, &audio, sample_rate, frame_period);
        
        assert!(!output.is_empty());
        println!("Input length: {}, Output length: {}", audio.len(), output.len());
    }
}
