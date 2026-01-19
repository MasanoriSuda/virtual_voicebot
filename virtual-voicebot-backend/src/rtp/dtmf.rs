const SAMPLE_RATE: f64 = 8000.0;
const MIN_SIGNAL_ENERGY: f64 = 1.0e6;
const MIN_TONE_FRAMES: u8 = 2;
const LOW_FREQS: [f64; 4] = [697.0, 770.0, 852.0, 941.0];
const HIGH_FREQS: [f64; 4] = [1209.0, 1336.0, 1477.0, 1633.0];
const DTMF_MAP: [[char; 4]; 4] = [
    ['1', '2', '3', 'A'],
    ['4', '5', '6', 'B'],
    ['7', '8', '9', 'C'],
    ['*', '0', '#', 'D'],
];
#[derive(Debug, Default)]
pub struct DtmfDetector {
    pending_digit: Option<char>,
    pending_count: u8,
    active_digit: Option<char>,
}
impl DtmfDetector {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn ingest_mulaw(&mut self, payload: &[u8]) -> Option<char> {
        let detected = detect_digit_from_mulaw(payload);
        if detected.is_some() && detected == self.active_digit {
            return None;
        }
        match detected {
            Some(digit) => {
                if self.pending_digit == Some(digit) {
                    self.pending_count = self.pending_count.saturating_add(1);
                } else {
                    self.pending_digit = Some(digit);
                    self.pending_count = 1;
                }
                if self.pending_count >= MIN_TONE_FRAMES {
                    self.active_digit = Some(digit);
                    self.pending_digit = None;
                    self.pending_count = 0;
                    return Some(digit);
                }
            }
            None => {
                self.pending_digit = None;
                self.pending_count = 0;
                self.active_digit = None;
            }
        }
        None
    }
}
fn detect_digit_from_mulaw(payload: &[u8]) -> Option<char> {
    if payload.len() < 80 {
        return None;
    }
    let mut samples: Vec<i16> = Vec::with_capacity(payload.len());
    let mut energy_sum = 0.0;
    for &b in payload {
        let sample = mulaw_to_linear16(b);
        energy_sum += (sample as f64) * (sample as f64);
        samples.push(sample);
    }
    let avg_energy = energy_sum / samples.len() as f64;
    if avg_energy < MIN_SIGNAL_ENERGY {
        return None;
    }
    let mut low = [0.0; 4];
    let mut high = [0.0; 4];
    for (idx, freq) in LOW_FREQS.iter().enumerate() {
        low[idx] = goertzel_power(&samples, *freq);
    }
    for (idx, freq) in HIGH_FREQS.iter().enumerate() {
        high[idx] = goertzel_power(&samples, *freq);
    }
    let (low_idx, low_max, low_second) = max_and_second(&low);
    let (high_idx, high_max, high_second) = max_and_second(&high);
    let total: f64 = low.iter().sum::<f64>() + high.iter().sum::<f64>();
    if total <= 0.0 {
        return None;
    }
    if low_max < low_second * 2.5 || high_max < high_second * 2.5 {
        return None;
    }
    if low_max < total * 0.1 || high_max < total * 0.1 {
        return None;
    }
    Some(DTMF_MAP[low_idx][high_idx])
}
fn goertzel_power(samples: &[i16], freq: f64) -> f64 {
    let n = samples.len() as f64;
    let k = (0.5 + (n * freq / SAMPLE_RATE)).floor();
    let w = 2.0 * std::f64::consts::PI * k / n;
    let coeff = 2.0 * w.cos();
    let mut s_prev = 0.0;
    let mut s_prev2 = 0.0;
    for &sample in samples {
        let s = (sample as f64) + coeff * s_prev - s_prev2;
        s_prev2 = s_prev;
        s_prev = s;
    }
    s_prev2 * s_prev2 + s_prev * s_prev - coeff * s_prev * s_prev2
}
fn max_and_second(values: &[f64; 4]) -> (usize, f64, f64) {
    let mut max_idx = 0;
    let mut max_val = values[0];
    let mut second_val = 0.0;
    for (idx, &val) in values.iter().enumerate().skip(1) {
        if val > max_val {
            second_val = max_val;
            max_val = val;
            max_idx = idx;
        } else if val > second_val {
            second_val = val;
        }
    }
    (max_idx, max_val, second_val)
}
fn mulaw_to_linear16(mu: u8) -> i16 {
    const BIAS: i16 = 0x84;
    let mu = !mu;
    let sign = (mu & 0x80) != 0;
    let segment = (mu & 0x70) >> 4;
    let mantissa = mu & 0x0F;
    let mut value = ((mantissa as i16) << 4) + 0x08;
    value <<= segment as i16;
    value -= BIAS;
    if sign { -value } else { value }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn generate_tone(low: f64, high: f64, duration_ms: u32) -> Vec<u8> {
        let sample_count = (SAMPLE_RATE as u32 * duration_ms / 1000) as usize;
        let mut out = Vec::with_capacity(sample_count);
        for n in 0..sample_count {
            let t = n as f64 / SAMPLE_RATE;
            let sample = (16000.0 * (2.0 * std::f64::consts::PI * low * t).sin()
                + 16000.0 * (2.0 * std::f64::consts::PI * high * t).sin())
                / 2.0;
            out.push(linear16_to_mulaw(sample as i16));
        }
        out
    }
    fn linear16_to_mulaw(sample: i16) -> u8 {
        const BIAS: i32 = 0x84;
        let mut pcm = sample as i32;
        let sign = if pcm < 0 {
            pcm = -pcm;
            0x80
        } else {
            0x00
        };
        pcm += BIAS;
        if pcm > 0x7FFF {
            pcm = 0x7FFF;
        }
        let mut exponent: u32 = 7;
        for exp in 0u32..8 {
            if pcm <= (0x1F << (exp + 3)) {
                exponent = exp;
                break;
            }
        }
        let mantissa = ((pcm >> (exponent + 3)) & 0x0F) as u8;
        !(sign | ((exponent as u8) << 4) | mantissa)
    }
    fn detect_tone(tone: &[u8]) -> Option<char> {
        let mut detector = DtmfDetector::new();
        tone.chunks(160).find_map(|chunk| detector.ingest_mulaw(chunk))
    }
    #[test]
    fn detects_dtmf_one() {
        assert_eq!(detect_tone(&generate_tone(697.0, 1209.0, 100)), Some('1'));
    }
    #[test]
    fn detects_dtmf_hash() {
        assert_eq!(detect_tone(&generate_tone(941.0, 1477.0, 100)), Some('#'));
    }
}
