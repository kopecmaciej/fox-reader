use std::f32::consts::PI;

/// Generates a Hann window of a given size.
///
/// The Hann window is defined as:
/// w(n) = 0.5 * (1.0 - cos(2π * n / (N - 1)))
fn hann_window(size: usize) -> Vec<f32> {
    (0..size)
        .map(|i| 0.5 * (1.0 - (2.0 * PI * i as f32 / (size - 1) as f32).cos()))
        .collect()
}

/// A normalized cross–correlation function.
///
/// Returns a value in [-1.0, 1.0] indicating how similar the two slices are.
fn normalized_cross_correlation(a: &[f32], b: &[f32]) -> f32 {
    let epsilon = 1e-9;
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let energy_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let energy_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot / ((energy_a * energy_b).max(epsilon))
}

/// A normalized WSOLA (Waveform Similarity Overlap–Add) time–stretching function.
///
/// # Arguments
///
/// * `input` - Input audio samples (mono, f32)
/// * `speed` - Speed factor (> 0). E.g., 1.5 means 50% faster playback.
/// * `search_range` - Number of samples to search (both directions) for best overlap.
///
/// # Returns
///
/// A vector containing the processed audio samples.
pub fn wsola_normalized(input: &[f32], speed: f32, search_range: usize) -> Vec<f32> {
    if input.len() < 2048 {
        return input.to_vec();
    }

    // Configuration parameters.
    let window_size = 2048;
    let analysis_hop = 1568;
    let synthesis_hop = (analysis_hop as f32 / speed).round() as usize;

    // Estimate the output length.
    let out_len = (input.len() as f32 / speed).ceil() as usize + window_size;
    let mut output = vec![0.0; out_len];
    let mut norm = vec![0.0; out_len]; // Buffer for window contribution (normalization)
    let window = hann_window(window_size);

    // Process the first frame (direct copy with windowing).
    let mut analysis_pos = 0;
    let mut synthesis_pos = 0;
    for i in 0..window_size {
        if analysis_pos + i < input.len() && synthesis_pos + i < output.len() {
            let w = window[i];
            output[synthesis_pos + i] = input[analysis_pos + i] * w;
            norm[synthesis_pos + i] = w;
        }
    }
    analysis_pos += analysis_hop;
    synthesis_pos += synthesis_hop;

    // Process subsequent frames.
    while analysis_pos + window_size < input.len() && synthesis_pos + window_size < output.len() {
        let overlap_length = synthesis_hop.min(window_size);
        // Define the region in the output where the previous frame overlaps.
        let prev_overlap_start = synthesis_pos.saturating_sub(overlap_length);
        let prev_overlap = &output[prev_overlap_start..synthesis_pos];

        // Search for the best alignment by maximizing normalized cross–correlation.
        let mut best_offset = 0;
        let mut best_corr = -1.0; // normalized correlation lies in [-1, 1]
        for offset in -(search_range as isize)..=(search_range as isize) {
            // Candidate index (make sure we don't go negative).
            let candidate_index = match (analysis_pos as isize).checked_add(offset) {
                Some(idx) if idx >= 0 => idx as usize,
                _ => continue,
            };
            if candidate_index + overlap_length > input.len() {
                continue;
            }
            let candidate_overlap = &input[candidate_index..candidate_index + overlap_length];
            let corr = normalized_cross_correlation(prev_overlap, candidate_overlap);
            if corr > best_corr {
                best_corr = corr;
                best_offset = offset;
            }
        }
        let candidate_index = (analysis_pos as isize + best_offset) as usize;

        // Overlap–add: add the windowed candidate frame to the output and update normalization.
        for i in 0..window_size {
            if candidate_index + i < input.len() && synthesis_pos + i < output.len() {
                let w = window[i];
                output[synthesis_pos + i] += input[candidate_index + i] * w;
                norm[synthesis_pos + i] += w;
            }
        }
        analysis_pos += analysis_hop;
        synthesis_pos += synthesis_hop;
    }

    // Normalize the output by dividing by the summed window contributions.
    for i in 0..output.len() {
        if norm[i].abs() > 1e-6 {
            output[i] /= norm[i];
        }
    }
    output
}
