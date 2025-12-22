use crate::rtp::payload::{classify_payload, PayloadKind, UnsupportedPayload};

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

fn linear16_to_mulaw(sample: i16) -> u8 {
    const BIAS: i16 = 0x84;
    const MAX: i16 = 0x7FFF;

    let mut pcm = sample;
    let sign = if pcm < 0 {
        pcm = -pcm;
        0x80
    } else {
        0x00
    };
    if pcm > MAX {
        pcm = MAX;
    }
    pcm += BIAS;

    let mut exponent = 7;
    let mut mask = 0x4000;
    while exponent > 0 && (pcm & mask) == 0 {
        exponent -= 1;
        mask >>= 1;
    }
    let mantissa = ((pcm >> (exponent + 3)) & 0x0F) as u8;
    let mu = !(sign | ((exponent as u8) << 4) | mantissa);
    mu
}

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
    if sign { -value } else { value }
}

fn linear16_to_alaw(sample: i16) -> u8 {
    let mut pcm = sample as i32;
    let sign = if pcm >= 0 {
        0x80
    } else {
        pcm = -pcm - 1;
        0x00
    };
    if pcm > 0x7FFF {
        pcm = 0x7FFF;
    }

    let mut exponent: u8 = 0;
    let mut mask: i32 = 0x400;
    while exponent < 7 && (pcm & mask) == 0 {
        exponent += 1;
        mask >>= 1;
    }
    let mantissa = if exponent == 0 {
        (pcm >> 4) & 0x0F
    } else {
        (pcm >> (exponent + 3)) & 0x0F
    } as u8;

    let alaw = sign | (exponent << 4) | mantissa;
    alaw ^ 0x55
}
