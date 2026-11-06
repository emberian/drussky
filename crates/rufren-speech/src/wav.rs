use crate::*;

/// Parse only bounded mono 24 kHz signed 16-bit little-endian PCM RIFF/WAVE.
/// Unknown container chunks are skipped safely; encoded originals are canonicalized.
pub(crate) fn decode(bytes: &[u8]) -> Result<&[u8], SpeechError> {
    if bytes.len() > MAX_PCM_BYTES + 4096 {
        return Err(SpeechError::ResponseTooLarge);
    }
    if bytes.len() < 44 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(SpeechError::InvalidResponse);
    }
    let read_u32 =
        |offset| u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
    if read_u32(4).checked_add(8) != Some(bytes.len()) {
        return Err(SpeechError::InvalidResponse);
    }
    let mut offset = 12;
    let mut format = false;
    let mut data = None;
    while offset < bytes.len() {
        if bytes.len() - offset < 8 {
            return Err(SpeechError::InvalidResponse);
        }
        let length = read_u32(offset + 4);
        let start = offset + 8;
        let end = start
            .checked_add(length)
            .ok_or(SpeechError::InvalidResponse)?;
        if end > bytes.len() {
            return Err(SpeechError::InvalidResponse);
        }
        match &bytes[offset..offset + 4] {
            b"fmt " => {
                if format
                    || length < 16
                    || bytes[start..start + 2] != 1u16.to_le_bytes()
                    || bytes[start + 2..start + 4] != 1u16.to_le_bytes()
                    || read_u32(start + 4) != SAMPLE_RATE as usize
                    || read_u32(start + 8) != SAMPLE_RATE as usize * 2
                    || bytes[start + 12..start + 14] != 2u16.to_le_bytes()
                    || bytes[start + 14..start + 16] != 16u16.to_le_bytes()
                {
                    return Err(SpeechError::InvalidResponse);
                }
                format = true;
            }
            b"data" => {
                if data.is_some() {
                    return Err(SpeechError::InvalidResponse);
                }
                data = Some(&bytes[start..end]);
            }
            _ => (),
        }
        offset = end
            .checked_add(length % 2)
            .ok_or(SpeechError::InvalidResponse)?;
    }
    let data = data.ok_or(SpeechError::InvalidResponse)?;
    if !format
        || offset != bytes.len()
        || data.is_empty()
        || data.len() % 2 != 0
        || data.iter().all(|b| *b == 0)
    {
        return Err(SpeechError::InvalidResponse);
    }
    if data.len() > MAX_PCM_BYTES {
        return Err(SpeechError::ResponseTooLarge);
    }
    Ok(data)
}

pub(crate) fn encode(pcm: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(44 + pcm.len());
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + pcm.len() as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    out.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(pcm.len() as u32).to_le_bytes());
    out.extend_from_slice(pcm);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn speech_wave_container_bounds_and_format_are_checked() {
        let original = encode(&[1, 0, 2, 0]);
        assert_eq!(decode(&original).unwrap(), [1, 0, 2, 0]);
        for offset in [0, 4, 8, 20, 22, 24, 28, 32, 34, 40] {
            let mut bad = original.clone();
            bad[offset] ^= 1;
            assert!(decode(&bad).is_err(), "{offset}");
        }
        for len in 0..original.len() {
            assert!(decode(&original[..len]).is_err());
        }
        let mut malicious = original.clone();
        malicious[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(decode(&malicious).is_err());
        let mut container = original[..36].to_vec();
        container.extend_from_slice(b"JUNK");
        container.extend_from_slice(&1u32.to_le_bytes());
        container.extend_from_slice(&[42, 0]);
        container.extend_from_slice(&original[36..]);
        let size = container.len() as u32 - 8;
        container[4..8].copy_from_slice(&size.to_le_bytes());
        assert_eq!(encode(decode(&container).unwrap()), original);
    }
}
