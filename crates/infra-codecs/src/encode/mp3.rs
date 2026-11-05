//! Escritor MP3 sobre o LAME (`mp3lame-encoder`, compilado do código-fonte; LGPL — ADR-0022).
//!
//! Escreve o quadro de tag LAME/Xing no início em `finish`, o que informa atraso e preenchimento
//! do codificador: decodificadores com suporte a "gapless" (symphonia incluso) devolvem o
//! comprimento exato do original.

use super::Mp3Mode;
use mp3lame_encoder::{
    max_required_buffer_size, Bitrate, Builder, DualPcm, Encoder as Lame, FlushGap, Mode, MonoPcm,
    Quality, VbrMode,
};
use std::io::{Seek, SeekFrom, Write};
use std::num::NonZeroU32;

/// Taxas de saída suportadas pelo MPEG-1/2/2.5 Layer III.
pub(crate) const MP3_RATES: [u32; 9] = [
    8_000, 11_025, 12_000, 16_000, 22_050, 24_000, 32_000, 44_100, 48_000,
];

pub(crate) struct Mp3Writer<W: Write + Seek> {
    out: W,
    lame: Lame,
    channels: usize,
    buf: Vec<u8>,
}

fn err(e: impl std::fmt::Debug) -> String {
    format!("{e:?}")
}

fn bitrate(kbps: u16) -> Option<Bitrate> {
    Some(match kbps {
        128 => Bitrate::Kbps128,
        160 => Bitrate::Kbps160,
        192 => Bitrate::Kbps192,
        224 => Bitrate::Kbps224,
        256 => Bitrate::Kbps256,
        320 => Bitrate::Kbps320,
        _ => return None,
    })
}

/// Bitrates CBR aceitos (kbps).
pub const MP3_CBR_KBPS: [u16; 6] = [128, 160, 192, 224, 256, 320];

impl<W: Write + Seek> Mp3Writer<W> {
    pub(crate) fn new(
        out: W,
        sample_rate: u32,
        channels: usize,
        mode: Mp3Mode,
    ) -> Result<Self, String> {
        let mut b = Builder::new().ok_or("LAME não pôde ser inicializado")?;
        b.set_num_channels(channels as u8).map_err(err)?;
        b.set_sample_rate(sample_rate).map_err(err)?;
        b.set_output_sample_rate(NonZeroU32::new(sample_rate))
            .map_err(err)?;
        b.set_mode(if channels == 1 {
            Mode::Mono
        } else {
            Mode::JointStereo
        })
        .map_err(err)?;
        b.set_quality(Quality::NearBest).map_err(err)?;
        b.set_to_write_vbr_tag(true).map_err(err)?;
        match mode {
            Mp3Mode::Cbr(kbps) => {
                let br = bitrate(kbps).ok_or_else(|| format!("bitrate {kbps} kbps inválido"))?;
                b.set_vbr_mode(VbrMode::Off).map_err(err)?;
                b.set_brate(br).map_err(err)?;
            }
            Mp3Mode::V0 => {
                b.set_vbr_mode(VbrMode::Mtrh).map_err(err)?;
                b.set_vbr_quality(Quality::Best).map_err(err)?;
            }
        }
        let lame = b.build().map_err(err)?;
        Ok(Self {
            out,
            lame,
            channels,
            buf: Vec::new(),
        })
    }

    pub(crate) fn write(&mut self, block: &[&[f32]]) -> Result<(), String> {
        let frames = block.first().map_or(0, |c| c.len());
        if frames == 0 {
            return Ok(());
        }
        // Entrada IEEE float do LAME espera ±1,0; limita para não saturar o codificador.
        let clamp = |c: &[f32]| -> Vec<f32> { c.iter().map(|x| x.clamp(-1.0, 1.0)).collect() };
        self.buf.clear();
        self.buf
            .reserve(max_required_buffer_size(frames * self.channels));
        if self.channels == 1 {
            let mono = clamp(block[0]);
            self.lame
                .encode_to_vec(MonoPcm(&mono[..]), &mut self.buf)
                .map_err(err)?;
        } else {
            let (l, r) = (clamp(block[0]), clamp(block[1]));
            self.lame
                .encode_to_vec(
                    DualPcm {
                        left: &l[..],
                        right: &r[..],
                    },
                    &mut self.buf,
                )
                .map_err(err)?;
        }
        self.out.write_all(&self.buf).map_err(err)
    }

    pub(crate) fn finish(mut self) -> Result<W, String> {
        self.buf.clear();
        self.buf.reserve(7200 * 2);
        self.lame
            .flush_to_vec::<FlushGap>(&mut self.buf)
            .map_err(err)?;
        self.out.write_all(&self.buf).map_err(err)?;
        let mut tag = Vec::with_capacity(self.lame.lame_tag_size());
        if self.lame.lame_tag_encode_to_vec(&mut tag).is_some() {
            let end = self.out.stream_position().map_err(err)?;
            self.out.seek(SeekFrom::Start(0)).map_err(err)?;
            self.out.write_all(&tag).map_err(err)?;
            self.out.seek(SeekFrom::Start(end)).map_err(err)?;
        }
        self.out.flush().map_err(err)?;
        Ok(self.out)
    }
}
