use crate::{AudioError, MAX_WAV_BYTES, PcmAudio, validate_shape};

fn u16le(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn u32le(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

pub(crate) fn decode(bytes: &[u8]) -> Result<PcmAudio, AudioError> {
    if bytes.len() > MAX_WAV_BYTES {
        return Err(AudioError::TooLarge);
    }
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(AudioError::Wav("expected RIFF/WAVE header"));
    }
    let end = (u32le(bytes, 4) as usize)
        .checked_add(8)
        .ok_or(AudioError::Wav("size overflow"))?;
    if end != bytes.len() {
        return Err(AudioError::Wav("RIFF length does not match file"));
    }
    let mut format = None;
    let mut data = None;
    let mut offset = 12;
    while offset < end {
        if end - offset < 8 {
            return Err(AudioError::Wav("truncated chunk header"));
        }
        let tag = &bytes[offset..offset + 4];
        let size = u32le(bytes, offset + 4) as usize;
        offset += 8;
        let next = offset
            .checked_add(size)
            .filter(|n| *n <= end)
            .ok_or(AudioError::Wav("truncated chunk"))?;
        match tag {
            b"fmt " => {
                if format.is_some() || size < 16 {
                    return Err(AudioError::Wav("duplicate or short format chunk"));
                }
                format = Some(&bytes[offset..next]);
            }
            b"data" => {
                if data.is_some() {
                    return Err(AudioError::Wav("multiple data chunks"));
                }
                data = Some(&bytes[offset..next]);
            }
            _ => {}
        }
        offset = next
            .checked_add(size % 2)
            .filter(|n| *n <= end)
            .ok_or(AudioError::Wav("missing chunk padding"))?;
    }
    let fmt = format.ok_or(AudioError::Wav("missing format chunk"))?;
    let data = data.ok_or(AudioError::Wav("missing data chunk"))?;
    if fmt.len() != 16 && (fmt.len() != 18 || u16le(fmt, 16) != 0) {
        return Err(AudioError::Wav("unsupported format extension"));
    }
    let encoding = u16le(fmt, 0);
    let channels = u16le(fmt, 2);
    let sample_rate = u32le(fmt, 4);
    let bits = u16le(fmt, 14);
    if !matches!((encoding, bits), (1, 8 | 16 | 24 | 32) | (3, 32)) {
        return Err(AudioError::Wav("requires integer PCM or IEEE float32"));
    }
    if !(1..=8).contains(&channels) || !(8_000..=192_000).contains(&sample_rate) {
        return Err(AudioError::Wav("unsupported channels or sample rate"));
    }
    let bytes_per_sample = bits as usize / 8;
    let block_align = channels as usize * bytes_per_sample;
    if u16le(fmt, 12) as usize != block_align
        || u32le(fmt, 8) != sample_rate * block_align as u32
        || data.len() % block_align != 0
    {
        return Err(AudioError::Wav("inconsistent block alignment or byte rate"));
    }
    validate_shape(sample_rate, channels, data.len() / bytes_per_sample)?;
    let samples = data
        .chunks_exact(bytes_per_sample)
        .map(|x| match (encoding, bits) {
            (1, 8) => (x[0] as f32 - 128.0) / 128.0,
            (1, 16) => i16::from_le_bytes([x[0], x[1]]) as f32 / 32768.0,
            (1, 24) => ((i32::from_le_bytes([x[0], x[1], x[2], 0]) << 8) >> 8) as f32 / 8_388_608.0,
            (1, 32) => i32::from_le_bytes(x.try_into().unwrap()) as f32 / 2_147_483_648.0,
            (3, 32) => f32::from_le_bytes(x.try_into().unwrap()),
            _ => unreachable!("encoding checked before allocation"),
        })
        .collect();
    PcmAudio::from_interleaved(sample_rate, channels, samples)
}

pub(crate) fn encode(audio: &PcmAudio) -> Vec<u8> {
    let meta = audio.metadata();
    let data_size = audio.samples().len() as u32 * 4;
    let mut bytes = Vec::with_capacity(data_size as usize + 58);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(50 + data_size).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    // WAVEFORMATEX plus the fact chunk required for non-PCM formats.
    bytes.extend_from_slice(&18_u32.to_le_bytes());
    bytes.extend_from_slice(&3_u16.to_le_bytes());
    bytes.extend_from_slice(&meta.channels.to_le_bytes());
    bytes.extend_from_slice(&meta.sample_rate.to_le_bytes());
    bytes.extend_from_slice(&(meta.sample_rate * meta.channels as u32 * 4).to_le_bytes());
    bytes.extend_from_slice(&(meta.channels * 4).to_le_bytes());
    bytes.extend_from_slice(&32_u16.to_le_bytes());
    bytes.extend_from_slice(&0_u16.to_le_bytes());
    bytes.extend_from_slice(b"fact");
    bytes.extend_from_slice(&4_u32.to_le_bytes());
    bytes.extend_from_slice(&(meta.frames as u32).to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_size.to_le_bytes());
    for sample in audio.samples() {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    bytes
}
