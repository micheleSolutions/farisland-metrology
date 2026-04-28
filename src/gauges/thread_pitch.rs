/// # ThreadPitchGauge
///
/// Measures the pitch (distance between consecutive thread crests) of a screw thread
/// from a silhouette profile image.
///
/// ## Algorithm
/// 1. Extract a brightness profile along the thread axis (parallel to the thread)
/// 2. Smooth and compute gradient to find the periodic crest/valley pattern
/// 3. **Peak detection method**: find all crests (local maxima in brightness or gradient),
///    compute consecutive distances, return mean pitch ± std dev
/// 4. **FFT method** (optional, more robust): compute FFT of the profile, find the
///    dominant frequency peak → pitch = 1/frequency
///
/// Both methods are provided. The FFT method is more robust to partial occlusion,
/// noise, and non-uniform illumination. The peak method gives individual pitch
/// measurements useful for detecting single-pitch errors.

use crate::error::{MetrologyError, MetrologyResult};
use crate::geometry::Point2D;
use crate::image::GrayImageRef;
use crate::profile;

/// Configuration for thread pitch measurement.
#[derive(Debug, Clone)]
pub struct ThreadPitchGaugeConfig {
    /// Start point of the scan line (along the thread axis).
    pub start: Point2D,
    /// End point of the scan line.
    pub end: Point2D,
    /// Scan width for profile averaging (perpendicular to thread axis).
    pub scan_width: u32,
    /// Gaussian smoothing sigma.
    pub smoothing_sigma: f64,
    /// Minimum peak prominence (for peak detection method).
    pub min_peak_prominence: f64,
    /// Expected pitch range [min, max] in pixels (for validation).
    pub expected_pitch_range: (f64, f64),
    /// Sampling step.
    pub step: f64,
}

impl Default for ThreadPitchGaugeConfig {
    fn default() -> Self {
        Self {
            start: Point2D::new(0.0, 0.0),
            end: Point2D::new(500.0, 0.0),
            scan_width: 10,
            smoothing_sigma: 2.0,
            min_peak_prominence: 5.0,
            expected_pitch_range: (5.0, 100.0),
            step: 1.0,
        }
    }
}

/// Result of a thread pitch measurement.
#[derive(Debug, Clone)]
pub struct ThreadPitchResult {
    /// Mean pitch in pixels.
    pub mean_pitch_px: f64,
    /// Standard deviation of pitch measurements.
    pub std_dev_px: f64,
    /// Individual pitch values (distance between consecutive crests).
    pub pitches: Vec<f64>,
    /// Positions of detected crests along the profile (in pixels from start).
    pub crest_positions: Vec<f64>,
    /// Number of complete threads measured.
    pub thread_count: usize,
}

/// Result of FFT-based pitch measurement.
#[derive(Debug, Clone)]
pub struct ThreadPitchFftResult {
    /// Dominant pitch in pixels.
    pub pitch_px: f64,
    /// Magnitude of the dominant frequency (confidence indicator).
    pub dominant_magnitude: f64,
    /// Second-strongest pitch candidate (for quality assessment).
    pub secondary_pitch_px: Option<f64>,
}

pub struct ThreadPitchGauge;

impl ThreadPitchGauge {
    /// Measure thread pitch using the peak detection method.
    ///
    /// This detects individual crests in the brightness profile and measures
    /// the distance between consecutive crests.
    pub fn measure_by_peaks(
        image: &GrayImageRef<'_>,
        config: &ThreadPitchGaugeConfig,
    ) -> MetrologyResult<ThreadPitchResult> {
        let profile = profile::extract_profile(
            image,
            config.start,
            config.end,
            config.scan_width,
            config.step,
        );

        if profile.len() < 10 {
            return Err(MetrologyError::EmptyProfile);
        }

        let smoothed = profile::smooth_gaussian(&profile.values, config.smoothing_sigma);

        // Find crests (local maxima with sufficient prominence)
        let crests = find_crests_subpixel(&smoothed, config.min_peak_prominence);

        if crests.len() < 2 {
            return Err(MetrologyError::InsufficientData {
                needed: 2,
                got: crests.len(),
            });
        }

        // Compute consecutive pitch values
        let mut pitches = Vec::with_capacity(crests.len() - 1);
        for i in 1..crests.len() {
            let pitch = (crests[i] - crests[i - 1]) * config.step;
            // Filter by expected range
            if pitch >= config.expected_pitch_range.0 && pitch <= config.expected_pitch_range.1 {
                pitches.push(pitch);
            }
        }

        if pitches.is_empty() {
            return Err(MetrologyError::NoEdgePairFound);
        }

        let mean = pitches.iter().sum::<f64>() / pitches.len() as f64;
        let variance =
            pitches.iter().map(|p| (p - mean) * (p - mean)).sum::<f64>() / pitches.len() as f64;
        let std_dev = variance.sqrt();

        let crest_positions: Vec<f64> = crests.iter().map(|c| c * config.step).collect();

        Ok(ThreadPitchResult {
            mean_pitch_px: mean,
            std_dev_px: std_dev,
            pitches,
            crest_positions,
            thread_count: crests.len() - 1,
        })
    }

    /// Measure thread pitch using FFT (frequency domain analysis).
    ///
    /// More robust than peak detection for noisy or partially occluded threads.
    /// Computes the real DFT, finds the dominant frequency in the expected pitch range,
    /// and converts to pitch = N*step/frequency_bin.
    pub fn measure_by_fft(
        image: &GrayImageRef<'_>,
        config: &ThreadPitchGaugeConfig,
    ) -> MetrologyResult<ThreadPitchFftResult> {
        let profile = profile::extract_profile(
            image,
            config.start,
            config.end,
            config.scan_width,
            config.step,
        );

        if profile.len() < 10 {
            return Err(MetrologyError::EmptyProfile);
        }

        let smoothed = profile::smooth_gaussian(&profile.values, config.smoothing_sigma);

        // Remove DC component (mean)
        let mean_val = smoothed.iter().sum::<f64>() / smoothed.len() as f64;
        let centered: Vec<f64> = smoothed.iter().map(|v| v - mean_val).collect();

        let n = centered.len();

        // Compute magnitude spectrum via DFT (real input, O(N*N/2) — acceptable for
        // typical profile lengths of hundreds to low thousands of samples).
        // For profiles >4096 samples, a proper FFT (rustfft crate) would be better,
        // but we avoid external dependencies in the core library.
        let half_n = n / 2;
        let mut magnitudes = Vec::with_capacity(half_n);
        let mut frequencies = Vec::with_capacity(half_n);

        // Frequency bins corresponding to expected pitch range
        let min_freq_bin =
            ((n as f64 * config.step) / config.expected_pitch_range.1).ceil() as usize;
        let max_freq_bin =
            ((n as f64 * config.step) / config.expected_pitch_range.0).floor() as usize;
        let min_bin = min_freq_bin.max(1);
        let max_bin = max_freq_bin.min(half_n);

        for k in min_bin..=max_bin {
            let freq = 2.0 * std::f64::consts::PI * k as f64 / n as f64;
            let mut re = 0.0;
            let mut im = 0.0;
            for (i, val) in centered.iter().enumerate() {
                let phase = freq * i as f64;
                re += val * phase.cos();
                im -= val * phase.sin();
            }
            let mag = (re * re + im * im).sqrt();
            magnitudes.push(mag);
            frequencies.push(k);
        }

        if magnitudes.is_empty() {
            return Err(MetrologyError::NoEdgeFound);
        }

        // Find dominant frequency
        let (best_idx, &best_mag) = magnitudes
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .unwrap();

        let best_bin = frequencies[best_idx];
        let pitch_px = (n as f64 * config.step) / best_bin as f64;

        // Parabolic interpolation on the magnitude spectrum for sub-bin precision
        let refined_pitch = if best_idx > 0 && best_idx < magnitudes.len() - 1 {
            let prev = magnitudes[best_idx - 1];
            let curr = magnitudes[best_idx];
            let next = magnitudes[best_idx + 1];
            let denom = prev - 2.0 * curr + next;
            if denom.abs() > 1e-10 {
                let delta = 0.5 * (prev - next) / denom;
                let refined_bin = best_bin as f64 + delta;
                (n as f64 * config.step) / refined_bin
            } else {
                pitch_px
            }
        } else {
            pitch_px
        };

        // Find secondary peak
        let secondary = magnitudes
            .iter()
            .enumerate()
            .filter(|(i, _)| (*i as isize - best_idx as isize).unsigned_abs() > 2)
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(i, _)| (n as f64 * config.step) / frequencies[i] as f64);

        Ok(ThreadPitchFftResult {
            pitch_px: refined_pitch,
            dominant_magnitude: best_mag,
            secondary_pitch_px: secondary,
        })
    }
}

/// Find local maxima (crests) with sub-pixel refinement via parabolic interpolation.
///
/// A crest must have prominence >= `min_prominence` (height above the higher of
/// the two neighboring valleys).
fn find_crests_subpixel(profile: &[f64], min_prominence: f64) -> Vec<f64> {
    let n = profile.len();
    if n < 3 {
        return Vec::new();
    }

    let mut crests = Vec::new();

    for i in 1..n - 1 {
        if profile[i] > profile[i - 1] && profile[i] > profile[i + 1] {
            // Check prominence: find the nearest valleys on each side
            let left_valley = find_valley_left(profile, i);
            let right_valley = find_valley_right(profile, i);
            let prominence = profile[i] - left_valley.max(right_valley);

            if prominence >= min_prominence {
                // Sub-pixel refinement
                let prev = profile[i - 1];
                let curr = profile[i];
                let next = profile[i + 1];
                let denom = prev - 2.0 * curr + next;
                let offset = if denom.abs() > 1e-10 {
                    0.5 * (prev - next) / denom
                } else {
                    0.0
                };
                crests.push(i as f64 + offset);
            }
        }
    }

    crests
}

fn find_valley_left(profile: &[f64], from: usize) -> f64 {
    let mut min_val = profile[from];
    for i in (0..from).rev() {
        if profile[i] < min_val {
            min_val = profile[i];
        }
        if profile[i] > min_val + 1.0 {
            break; // Rising again, found the valley
        }
    }
    min_val
}

fn find_valley_right(profile: &[f64], from: usize) -> f64 {
    let mut min_val = profile[from];
    for i in from + 1..profile.len() {
        if profile[i] < min_val {
            min_val = profile[i];
        }
        if profile[i] > min_val + 1.0 {
            break;
        }
    }
    min_val
}
