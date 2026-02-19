use crate::protocol::rtp::payload::{classify_payload, PayloadKind, UnsupportedPayload};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Codec {
    Pcmu,
    Pcma,
}

pub fn codec_from_pt(pt: u8) -> Result<Codec, UnsupportedPayload> {
    match classify_payload(pt)? {
        PayloadKind::Pcmu => Ok(Codec::Pcmu),
        PayloadKind::Pcma => Ok(Codec::Pcma),
    }
}

pub fn decode_to_mulaw(codec: Codec, payload: &[u8]) -> Vec<u8> {
    match codec {
        Codec::Pcmu => payload.to_vec(),
        Codec::Pcma => payload
            .iter()
            .map(|&a| linear16_to_mulaw(alaw_to_linear16(a)))
            .collect(),
    }
}

pub fn encode_from_mulaw(codec: Codec, payload: &[u8]) -> Vec<u8> {
    match codec {
        Codec::Pcmu => payload.to_vec(),
        Codec::Pcma => payload
            .iter()
            .map(|&mu| linear16_to_alaw(mulaw_to_linear16(mu)))
            .collect(),
    }
}

pub(crate) fn mulaw_to_linear16(mu: u8) -> i16 {
    const BIAS: i16 = 0x84;
    let mu = !mu;
    let sign = (mu & 0x80) != 0;
    let segment = (mu & 0x70) >> 4;
    let mantissa = mu & 0x0F;

    let mut value = ((mantissa as i16) << 3) + BIAS;
    value <<= segment as i16;
    if sign {
        BIAS - value
    } else {
        value - BIAS
    }
}

fn linear16_to_mulaw(sample: i16) -> u8 {
    const BIAS: i16 = 0x84;
    const CLIP: i16 = 8159;
    let mut pcm = sample >> 2;
    let mask = if pcm < 0 {
        pcm = -pcm;
        0x7F
    } else {
        0xFF
    };
    if pcm > CLIP {
        pcm = CLIP;
    }
    pcm += BIAS >> 2;

    let seg = search_g711_segment(pcm, &SEG_UEND);
    if seg >= 8 {
        return 0x7F ^ mask;
    }
    let uval = (seg << 4) | (((pcm >> (seg + 1)) & 0x0F) as u8);
    uval ^ mask
}

/// Converts an 8-bit A-law encoded value into a 16-bit linear PCM sample.
///
/// # Examples
///
/// ```ignore
/// let pcm: i16 = 1000;
/// let a = linear16_to_alaw(pcm);
/// assert_eq!(alaw_to_linear16(a), pcm);
/// ```
fn alaw_to_linear16(a: u8) -> i16 {
    let a = a ^ 0x55;
    let sign = (a & 0x80) != 0;
    let exponent = (a & 0x70) >> 4;
    let mantissa = a & 0x0F;

    let mut value = (mantissa as i16) << 4;
    value += 8;
    if exponent != 0 {
        value += 0x100;
        value <<= exponent - 1;
    }
    if sign {
        value
    } else {
        -value
    }
}

/// Converts a 16-bit linear PCM sample to its G.711 A-law encoded byte.
///
/// The input sample is clamped to the 16-bit positive range, mapped to sign,
/// exponent and mantissa fields, assembled into an A-law byte, and then
/// XOR-masked with 0x55 to produce the final encoded value.
///
/// # Examples
///
/// ```ignore
/// let encoded = linear16_to_alaw(0i16);
/// // `encoded` is the A-law representation of the silent PCM sample
/// let _ = encoded;
/// ```
fn linear16_to_alaw(sample: i16) -> u8 {
    let mut pcm = sample >> 3;
    let mask = if pcm >= 0 {
        0xD5
    } else {
        pcm = -pcm - 1;
        0x55
    };
    let seg = search_g711_segment(pcm, &SEG_AEND);
    if seg >= 8 {
        return 0x7F ^ mask;
    }
    let mut aval = seg << 4;
    if seg < 2 {
        aval |= ((pcm >> 1) & 0x0F) as u8;
    } else {
        aval |= ((pcm >> seg) & 0x0F) as u8;
    }
    aval ^ mask
}

const SEG_AEND: [i16; 8] = [0x1F, 0x3F, 0x7F, 0xFF, 0x1FF, 0x3FF, 0x7FF, 0xFFF];
const SEG_UEND: [i16; 8] = [0x3F, 0x7F, 0xFF, 0x1FF, 0x3FF, 0x7FF, 0xFFF, 0x1FFF];

fn search_g711_segment(value: i16, table: &[i16; 8]) -> u8 {
    for (idx, limit) in table.iter().enumerate() {
        if value <= *limit {
            return idx as u8;
        }
    }
    8
}

#[cfg(test)]
mod tests {
    use super::{
        alaw_to_linear16, codec_from_pt, decode_to_mulaw, encode_from_mulaw, linear16_to_alaw,
        linear16_to_mulaw, mulaw_to_linear16, Codec,
    };

    extern "C" {
        fn g711_linear2alaw(pcm_val: i16) -> u8;
        fn g711_alaw2linear(a_val: u8) -> i16;
        fn g711_linear2ulaw(pcm_val: i16) -> u8;
        fn g711_ulaw2linear(u_val: u8) -> i16;
    }

    fn ref_linear2alaw(sample: i16) -> u8 {
        unsafe { g711_linear2alaw(sample) }
    }

    fn ref_alaw2linear(code: u8) -> i16 {
        unsafe { g711_alaw2linear(code) }
    }

    fn ref_linear2ulaw(sample: i16) -> u8 {
        unsafe { g711_linear2ulaw(sample) }
    }

    fn ref_ulaw2linear(code: u8) -> i16 {
        unsafe { g711_ulaw2linear(code) }
    }

    #[test]
    fn codec_from_pt_maps_supported_payload_types() {
        assert_eq!(codec_from_pt(0).expect("pt=0 should be PCMU"), Codec::Pcmu);
        assert_eq!(codec_from_pt(8).expect("pt=8 should be PCMA"), Codec::Pcma);
    }

    #[test]
    fn codec_from_pt_rejects_unsupported_payload_type() {
        assert!(codec_from_pt(96).is_err());
    }

    #[test]
    fn decode_to_mulaw_pcmu_is_passthrough() {
        let payload = [0x00, 0x7F, 0x80, 0xFF];
        assert_eq!(decode_to_mulaw(Codec::Pcmu, &payload), payload);
    }

    #[test]
    fn encode_from_mulaw_pcmu_is_passthrough() {
        let payload = [0x00, 0x7F, 0x80, 0xFF];
        assert_eq!(encode_from_mulaw(Codec::Pcmu, &payload), payload);
    }

    #[test]
    fn pcma_decode_encode_matches_scalar_conversion_for_all_codewords() {
        let payload: Vec<u8> = (u8::MIN..=u8::MAX).collect();
        let decoded = decode_to_mulaw(Codec::Pcma, &payload);
        assert_eq!(decoded.len(), payload.len());
        for (&a, &mu) in payload.iter().zip(decoded.iter()) {
            assert_eq!(mu, linear16_to_mulaw(alaw_to_linear16(a)));
        }

        let encoded = encode_from_mulaw(Codec::Pcma, &decoded);
        assert_eq!(encoded.len(), decoded.len());
        for (&mu, &a) in decoded.iter().zip(encoded.iter()) {
            assert_eq!(a, linear16_to_alaw(mulaw_to_linear16(mu)));
        }
    }

    #[test]
    fn mulaw_reencode_is_stable_for_all_linear16_values() {
        for sample in i16::MIN..=i16::MAX {
            let mu = linear16_to_mulaw(sample);
            let reconstructed = mulaw_to_linear16(mu);
            let reencoded = linear16_to_mulaw(reconstructed);
            assert_eq!(
                mulaw_to_linear16(reencoded),
                reconstructed,
                "mu-law reencode mismatch: sample={sample}, mu={mu:#04x}, reconstructed={reconstructed}, reencoded={reencoded:#04x}"
            );
        }
    }

    #[test]
    fn alaw_reencode_is_stable_for_all_linear16_values() {
        for sample in i16::MIN..=i16::MAX {
            let a = linear16_to_alaw(sample);
            let reconstructed = alaw_to_linear16(a);
            let reencoded = linear16_to_alaw(reconstructed);
            assert_eq!(
                reencoded, a,
                "a-law reencode mismatch: sample={sample}, a={a:#04x}, reconstructed={reconstructed}, reencoded={reencoded:#04x}"
            );
        }
    }

    #[test]
    fn mulaw_encode_matches_g711_reference_for_all_linear16_values() {
        for sample in i16::MIN..=i16::MAX {
            let actual = linear16_to_mulaw(sample);
            let expected = ref_linear2ulaw(sample);
            assert_eq!(
                actual, expected,
                "mu-law encode mismatch: sample={sample}, actual={actual:#04x}, expected={expected:#04x}"
            );
        }
    }

    #[test]
    fn alaw_encode_matches_g711_reference_for_all_linear16_values() {
        for sample in i16::MIN..=i16::MAX {
            let actual = linear16_to_alaw(sample);
            let expected = ref_linear2alaw(sample);
            assert_eq!(
                actual, expected,
                "a-law encode mismatch: sample={sample}, actual={actual:#04x}, expected={expected:#04x}"
            );
        }
    }

    #[test]
    fn mulaw_decode_matches_g711_reference_for_all_codewords() {
        for code in u8::MIN..=u8::MAX {
            let actual = mulaw_to_linear16(code);
            let expected = ref_ulaw2linear(code);
            assert_eq!(
                actual, expected,
                "mu-law decode mismatch: code={code:#04x}, actual={actual}, expected={expected}"
            );
        }
    }

    #[test]
    fn alaw_decode_matches_g711_reference_for_all_codewords() {
        for code in u8::MIN..=u8::MAX {
            let actual = alaw_to_linear16(code);
            let expected = ref_alaw2linear(code);
            assert_eq!(
                actual, expected,
                "a-law decode mismatch: code={code:#04x}, actual={actual}, expected={expected}"
            );
        }
    }
}
