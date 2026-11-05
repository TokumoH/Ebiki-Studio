//! Escritor Ogg Vorbis sobre `vorbis_rs` (libvorbis aoTuV + libogg, BSD-3; compilado com `cc`).

use std::io::Write;
use std::num::{NonZeroU32, NonZeroU8};
use vorbis_rs::{VorbisBitrateManagementStrategy, VorbisEncoder, VorbisEncoderBuilder};

/// Tamanho máximo de bloco entregue ao libvorbis por chamada (recomendação da documentação).
const MAX_BLOCK: usize = 4096;

pub(crate) struct VorbisWriter<W: Write> {
    enc: VorbisEncoder<W>,
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

impl<W: Write> VorbisWriter<W> {
    /// `quality` em [-0,2; 1,0] (0,6 ≈ 192 kbps estéreo). `serial` fixa o número de série do
    /// fluxo Ogg (saída reprodutível).
    pub(crate) fn new(
        out: W,
        sample_rate: u32,
        channels: usize,
        quality: f32,
        serial: i32,
    ) -> Result<Self, String> {
        let rate = NonZeroU32::new(sample_rate).ok_or("taxa zero")?;
        let ch = u8::try_from(channels)
            .ok()
            .and_then(NonZeroU8::new)
            .ok_or("canais inválidos")?;
        let enc = VorbisEncoderBuilder::new_with_serial(rate, ch, out, serial)
            .bitrate_management_strategy(VorbisBitrateManagementStrategy::QualityVbr {
                target_quality: quality,
            })
            .build()
            .map_err(err)?;
        Ok(Self { enc })
    }

    pub(crate) fn write(&mut self, block: &[&[f32]]) -> Result<(), String> {
        let frames = block.first().map_or(0, |c| c.len());
        let mut start = 0;
        while start < frames {
            let end = (start + MAX_BLOCK).min(frames);
            let sub: Vec<&[f32]> = block.iter().map(|c| &c[start..end]).collect();
            self.enc.encode_audio_block(&sub).map_err(err)?;
            start = end;
        }
        Ok(())
    }

    pub(crate) fn finish(self) -> Result<W, String> {
        self.enc.finish().map_err(err)
    }
}
