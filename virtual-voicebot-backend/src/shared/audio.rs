const SEG_UEND: [i16; 8] = [0xFF, 0x1FF, 0x3FF, 0x7FF, 0xFFF, 0x1FFF, 0x3FFF, 0x7FFF];

/// Converts a 16-bit linear PCM sample to an 8-bit G.711 mu-law sample.
pub fn linear16_to_mulaw(sample: i16) -> u8 {
    const BIAS: i32 = 0x84;
    const CLIP: i32 = 32635;

    let mut pcm = sample as i32;
    let mask = if pcm < 0 {
        pcm = -pcm;
        0x7F
    } else {
        0xFF
    };
    if pcm > CLIP {
        pcm = CLIP;
    }
    pcm += BIAS;

    let seg = search_g711_segment(pcm as i16, &SEG_UEND);
    if seg >= 8 {
        return 0x7F ^ mask;
    }
    let uval = (seg << 4) | (((pcm >> (seg + 3)) & 0x0F) as u8);
    uval ^ mask
}

fn search_g711_segment(value: i16, table: &[i16; 8]) -> u8 {
    for (idx, limit) in table.iter().enumerate() {
        if value <= *limit {
            return idx as u8;
        }
    }
    8
}
