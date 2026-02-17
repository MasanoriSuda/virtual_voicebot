use std::path::Path;

use anyhow::{Context, Result};
use hound::{SampleFormat, WavReader, WavSpec, WavWriter};

pub fn merge_stereo_files(
    a_path: impl AsRef<Path>,
    b_path: impl AsRef<Path>,
    out_path: impl AsRef<Path>,
) -> Result<()> {
    let a_path = a_path.as_ref();
    let b_path = b_path.as_ref();
    let out_path = out_path.as_ref();

    let mut a_reader =
        WavReader::open(a_path).with_context(|| format!("open a-leg wav: {a_path:?}"))?;
    let mut b_reader =
        WavReader::open(b_path).with_context(|| format!("open b-leg wav: {b_path:?}"))?;

    let a_spec = a_reader.spec();
    let b_spec = b_reader.spec();
    if a_spec.channels != 2 || b_spec.channels != 2 {
        anyhow::bail!(
            "expected stereo inputs (a_leg={}ch, b_leg={}ch)",
            a_spec.channels,
            b_spec.channels
        );
    }
    if a_spec.sample_rate != b_spec.sample_rate {
        anyhow::bail!(
            "sample rate mismatch (a_leg={}Hz, b_leg={}Hz)",
            a_spec.sample_rate,
            b_spec.sample_rate
        );
    }
    if a_spec.bits_per_sample != 16
        || b_spec.bits_per_sample != 16
        || a_spec.sample_format != SampleFormat::Int
        || b_spec.sample_format != SampleFormat::Int
    {
        anyhow::bail!("expected 16-bit PCM input wav");
    }

    let out_spec = WavSpec {
        channels: 4,
        sample_rate: a_spec.sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut writer = WavWriter::create(out_path, out_spec)
        .with_context(|| format!("create merged wav: {out_path:?}"))?;

    let mut a_iter = a_reader.samples::<i16>();
    let mut b_iter = b_reader.samples::<i16>();

    loop {
        let a_l = read_sample(&mut a_iter)?;
        let a_r = read_sample(&mut a_iter)?;
        let b_l = read_sample(&mut b_iter)?;
        let b_r = read_sample(&mut b_iter)?;

        if a_l.is_none() && a_r.is_none() && b_l.is_none() && b_r.is_none() {
            break;
        }

        writer.write_sample(a_l.unwrap_or(0))?;
        writer.write_sample(a_r.unwrap_or(0))?;
        writer.write_sample(b_l.unwrap_or(0))?;
        writer.write_sample(b_r.unwrap_or(0))?;
    }

    writer.finalize()?;
    Ok(())
}

fn read_sample<I>(iter: &mut I) -> Result<Option<i16>>
where
    I: Iterator<Item = std::result::Result<i16, hound::Error>>,
{
    match iter.next() {
        Some(Ok(v)) => Ok(Some(v)),
        Some(Err(e)) => Err(e.into()),
        None => Ok(None),
    }
}
