//! `reelkit.audio.write`: a stereo buffer as a 16-bit PCM `.wav` file at 48,000 Hz.

use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

use crate::SR;

/// One sample as `reelkit.audio.write` stores it: `(np.clip(x, -1, 1) * 32767).astype(np.int16)`, that is
/// clipped to [-1, 1], scaled by 32,767 and cut toward zero (not rounded).
pub fn sample_i16(x: f64) -> i16 {
    (x.clamp(-1.0, 1.0) * 32767.0) as i16
}

/// The bytes of the file `scipy.io.wavfile.write(path, 48000, int16_frames)` writes: a 44-byte RIFF header (PCM,
/// 2 channels, 16 bits) and the frames, left sample first, little-endian.
pub fn wav16_bytes(x: &[[f64; 2]]) -> Vec<u8> {
    const CHANNELS: u16 = 2;
    const BYTES_PER_SAMPLE: u16 = 2;
    let block_align = CHANNELS * BYTES_PER_SAMPLE;
    let data_len = u32::try_from(x.len() * usize::from(block_align)).expect("a .wav holds at most 4 GiB");
    let rate = SR as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // WAVE_FORMAT_PCM
    out.extend_from_slice(&CHANNELS.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * u32::from(block_align)).to_le_bytes());
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&(8 * BYTES_PER_SAMPLE).to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for frame in x {
        for &s in frame {
            out.extend_from_slice(&sample_i16(s).to_le_bytes());
        }
    }
    out
}

/// `reelkit.audio.write(path, mix)`: writes the buffer as a 16-bit stereo `.wav` at 48,000 Hz.
pub fn write_wav16(path: &Path, x: &[[f64; 2]]) -> io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    file.write_all(&wav16_bytes(x))?;
    file.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_are_clipped_scaled_and_cut_toward_zero() {
        assert_eq!(sample_i16(1.0), 32767);
        assert_eq!(sample_i16(-1.0), -32767);
        assert_eq!(sample_i16(7.5), 32767);
        assert_eq!(sample_i16(-7.5), -32767);
        assert_eq!(sample_i16(0.0), 0);
        // 0.99999 * 32767 = 32766.67: cut, not rounded. The same below zero.
        assert_eq!(sample_i16(0.99999), 32766);
        assert_eq!(sample_i16(-0.99999), -32766);
        assert_eq!(sample_i16(1.5 / 32767.0), 1);
        assert_eq!(sample_i16(-1.5 / 32767.0), -1);
        assert_eq!(sample_i16(0.5 / 32767.0), 0);
        assert_eq!(sample_i16(-0.5 / 32767.0), 0);
    }

    #[test]
    fn the_file_is_a_canonical_pcm_wav() {
        let bytes = wav16_bytes(&[[1.0, -1.0], [0.5, 0.0], [-2.0, 0.25]]);
        #[rustfmt::skip]
        let want: [u8; 56] = [
            b'R', b'I', b'F', b'F', 48, 0, 0, 0, b'W', b'A', b'V', b'E',
            b'f', b'm', b't', b' ', 16, 0, 0, 0, 1, 0, 2, 0,
            0x80, 0xbb, 0, 0, 0x00, 0xee, 0x02, 0, 4, 0, 16, 0,
            b'd', b'a', b't', b'a', 12, 0, 0, 0,
            0xff, 0x7f, 0x01, 0x80, 0xff, 0x3f, 0, 0, 0x01, 0x80, 0xff, 0x1f,
        ];
        assert_eq!(bytes, want);
        assert_eq!(wav16_bytes(&[]).len(), 44);
    }

    #[test]
    fn write_puts_those_bytes_on_disk() {
        let path = std::env::temp_dir().join(format!("bf-sound-wav-test-{}.wav", std::process::id()));
        let x = [[0.25, -0.25], [1.0, -1.0]];
        write_wav16(&path, &x).unwrap();
        let on_disk = std::fs::read(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(on_disk, wav16_bytes(&x));
    }
}
