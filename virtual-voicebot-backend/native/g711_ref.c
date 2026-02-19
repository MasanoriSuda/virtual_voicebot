/*
 * Minimal CCITT G.711 reference functions used as test oracle.
 * Based on the commonly distributed Sun/CCITT g711.c implementation.
 */

#define SIGN_BIT (0x80)
#define QUANT_MASK (0x0F)
#define NSEGS (8)
#define SEG_SHIFT (4)
#define SEG_MASK (0x70)

static short seg_aend[8] = {0x1F, 0x3F, 0x7F, 0xFF,
                            0x1FF, 0x3FF, 0x7FF, 0xFFF};
static short seg_uend[8] = {0x3F, 0x7F, 0xFF, 0x1FF,
                            0x3FF, 0x7FF, 0xFFF, 0x1FFF};

static short search_segment(short val, short *table, short size) {
    short i;
    for (i = 0; i < size; i++) {
        if (val <= *table++) {
            return i;
        }
    }
    return size;
}

unsigned char g711_linear2alaw(short pcm_val) {
    short mask;
    short seg;
    unsigned char aval;

    pcm_val = pcm_val >> 3;

    if (pcm_val >= 0) {
        mask = 0xD5;
    } else {
        mask = 0x55;
        pcm_val = -pcm_val - 1;
    }

    seg = search_segment(pcm_val, seg_aend, NSEGS);

    if (seg >= NSEGS) {
        return (unsigned char) (0x7F ^ mask);
    }

    aval = (unsigned char) seg << SEG_SHIFT;
    if (seg < 2) {
        aval |= (pcm_val >> 1) & QUANT_MASK;
    } else {
        aval |= (pcm_val >> seg) & QUANT_MASK;
    }
    return (unsigned char) (aval ^ mask);
}

short g711_alaw2linear(unsigned char a_val) {
    short t;
    short seg;

    a_val ^= 0x55;
    t = (a_val & QUANT_MASK) << 4;
    seg = ((unsigned) a_val & SEG_MASK) >> SEG_SHIFT;

    switch (seg) {
    case 0:
        t += 8;
        break;
    case 1:
        t += 0x108;
        break;
    default:
        t += 0x108;
        t <<= seg - 1;
        break;
    }

    return (a_val & SIGN_BIT) ? t : -t;
}

#define BIAS (0x84)
#define CLIP (8159)

unsigned char g711_linear2ulaw(short pcm_val) {
    short mask;
    short seg;
    unsigned char uval;

    pcm_val = pcm_val >> 2;
    if (pcm_val < 0) {
        pcm_val = -pcm_val;
        mask = 0x7F;
    } else {
        mask = 0xFF;
    }

    if (pcm_val > CLIP) {
        pcm_val = CLIP;
    }
    pcm_val += (BIAS >> 2);

    seg = search_segment(pcm_val, seg_uend, NSEGS);

    if (seg >= NSEGS) {
        return (unsigned char) (0x7F ^ mask);
    }

    uval = (unsigned char) ((seg << 4) | ((pcm_val >> (seg + 1)) & 0x0F));
    return (unsigned char) (uval ^ mask);
}

short g711_ulaw2linear(unsigned char u_val) {
    short t;

    u_val = (unsigned char) ~u_val;
    t = ((u_val & QUANT_MASK) << 3) + BIAS;
    t <<= ((unsigned) u_val & SEG_MASK) >> SEG_SHIFT;

    return (u_val & SIGN_BIT) ? (BIAS - t) : (t - BIAS);
}
