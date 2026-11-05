//! Escritor AIFF (PCM 16/24 bits big-endian) e AIFF-C `fl32` (32 bits float), em fluxo.
//!
//! O cabeçalho é escrito com tamanhos provisórios e corrigido em `finish` (seek).

use std::io::{self, Seek, SeekFrom, Write};

pub(crate) struct AiffWriter<W: Write + Seek> {
    out: W,
    channels: u16,
    bits: u16,
    float: bool,
    frames: u64,
    /// Offset do campo `numSampleFrames` no COMM.
    frames_field: u64,
    /// Offset do campo de tamanho do SSND.
    ssnd_size_field: u64,
    data_bytes: u64,
}

/// Taxa de amostragem em ponto flutuante estendido de 80 bits (IEEE 754, big-endian).
pub(crate) fn extended_from_u32(rate: u32) -> [u8; 10] {
    let mut out = [0u8; 10];
    if rate == 0 {
        return out;
    }
    let exp = 31 - rate.leading_zeros(); // posição do bit mais significativo
    let biased = 16_383 + exp as u16;
    let mantissa = u64::from(rate) << (63 - exp);
    out[0..2].copy_from_slice(&biased.to_be_bytes());
    out[2..10].copy_from_slice(&mantissa.to_be_bytes());
    out
}

impl<W: Write + Seek> AiffWriter<W> {
    pub(crate) fn new(
        mut out: W,
        sample_rate: u32,
        channels: u16,
        bits: u16,
        float: bool,
    ) -> io::Result<Self> {
        let form_type: &[u8; 4] = if float { b"AIFC" } else { b"AIFF" };
        out.write_all(b"FORM")?;
        out.write_all(&0u32.to_be_bytes())?; // corrigido em finish
        out.write_all(form_type)?;
        if float {
            out.write_all(b"FVER")?;
            out.write_all(&4u32.to_be_bytes())?;
            out.write_all(&0xA280_5140u32.to_be_bytes())?; // AIFC versão 1
        }
        // COMM
        let comm_len: u32 = if float { 18 + 4 + 6 } else { 18 };
        out.write_all(b"COMM")?;
        out.write_all(&comm_len.to_be_bytes())?;
        out.write_all(&channels.to_be_bytes())?;
        let frames_field = out.stream_position()?;
        out.write_all(&0u32.to_be_bytes())?;
        out.write_all(&bits.to_be_bytes())?;
        out.write_all(&extended_from_u32(sample_rate))?;
        if float {
            out.write_all(b"fl32")?;
            // pstring "none" (contagem + 4 bytes) + pad para tamanho par: 1 + 4 + 1 = 6
            out.write_all(&[4, b'n', b'o', b'n', b'e', 0])?;
        }
        // SSND
        out.write_all(b"SSND")?;
        let ssnd_size_field = out.stream_position()?;
        out.write_all(&0u32.to_be_bytes())?;
        out.write_all(&0u32.to_be_bytes())?; // offset
        out.write_all(&0u32.to_be_bytes())?; // blockSize
        Ok(Self {
            out,
            channels,
            bits,
            float,
            frames: 0,
            frames_field,
            ssnd_size_field,
            data_bytes: 0,
        })
    }

    /// Escreve quadros intercalados já quantizados (`i32`) — usado para 16/24 bits.
    pub(crate) fn write_int_interleaved(&mut self, samples: &[i32]) -> io::Result<()> {
        let mut buf = Vec::with_capacity(samples.len() * 3);
        for &s in samples {
            if self.bits == 16 {
                buf.extend_from_slice(&(s as i16).to_be_bytes());
            } else {
                buf.extend_from_slice(&s.to_be_bytes()[1..4]);
            }
        }
        self.write_raw(&buf, samples.len())
    }

    pub(crate) fn write_float_interleaved(&mut self, samples: &[f32]) -> io::Result<()> {
        let mut buf = Vec::with_capacity(samples.len() * 4);
        for &s in samples {
            buf.extend_from_slice(&s.to_be_bytes());
        }
        self.write_raw(&buf, samples.len())
    }

    fn write_raw(&mut self, bytes: &[u8], samples: usize) -> io::Result<()> {
        debug_assert!(self.float || self.bits == 16 || self.bits == 24);
        self.out.write_all(bytes)?;
        self.data_bytes += bytes.len() as u64;
        self.frames += (samples / usize::from(self.channels)) as u64;
        Ok(())
    }

    pub(crate) fn finish(mut self) -> io::Result<W> {
        let too_big = || io::Error::other("arquivo AIFF excede 4 GiB");
        if self.data_bytes % 2 == 1 {
            self.out.write_all(&[0])?; // chunk com tamanho par
        }
        let end = self.out.stream_position()?;
        let form_size = u32::try_from(end - 8).map_err(|_| too_big())?;
        let ssnd_size = u32::try_from(self.data_bytes + 8).map_err(|_| too_big())?;
        let frames = u32::try_from(self.frames).map_err(|_| too_big())?;
        self.out.seek(SeekFrom::Start(4))?;
        self.out.write_all(&form_size.to_be_bytes())?;
        self.out.seek(SeekFrom::Start(self.frames_field))?;
        self.out.write_all(&frames.to_be_bytes())?;
        self.out.seek(SeekFrom::Start(self.ssnd_size_field))?;
        self.out.write_all(&ssnd_size.to_be_bytes())?;
        self.out.seek(SeekFrom::Start(end))?;
        self.out.flush()?;
        Ok(self.out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extended_float_of_common_rates() {
        // 44100 Hz = 0x400E AC44000000000000
        assert_eq!(
            extended_from_u32(44_100),
            [0x40, 0x0E, 0xAC, 0x44, 0, 0, 0, 0, 0, 0]
        );
        // 48000 Hz = 0x400E BB80000000000000
        assert_eq!(
            extended_from_u32(48_000),
            [0x40, 0x0E, 0xBB, 0x80, 0, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn header_sizes_are_patched() {
        let mut w = AiffWriter::new(io::Cursor::new(Vec::new()), 48_000, 1, 24, false).unwrap();
        w.write_int_interleaved(&[1, 2, 3]).unwrap(); // 9 bytes → pad
        let bytes = w.finish().unwrap().into_inner();
        let form_size = u32::from_be_bytes(bytes[4..8].try_into().unwrap()) as usize;
        assert_eq!(form_size, bytes.len() - 8);
        assert_eq!(bytes.len() % 2, 0);
    }
}
