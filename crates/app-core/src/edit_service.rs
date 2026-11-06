//! Casos de uso da Fase 3 — Edição / Pós-produção (RF-3.1 a RF-3.5).
//!
//! Finos: montam os comandos desfazíveis do contexto Project e os executam no histórico da
//! sessão. Nada aqui modifica arquivos de áudio (RNF-07): normalizar só mede a fonte decodificada
//! e define o ganho do clipe; o espectrograma é calculado pela porta de mídia (Pure Fabrication).

use crate::ports::SpectrogramRequest;
use crate::session::on_project;
use crate::studio::{AppError, AppResult, Studio};
use crate::view::SessionView;
use ctx_project::commands::{
    Crossfade, SetClipFades, SetClipGain, SetClipPolarity, SetClipReversed, SplitClip, TrimClip,
};
use ctx_project::{ClipId, FadeShape};
use serde::{Deserialize, Serialize};
use shared_kernel::id::AudioSourceId;
use shared_kernel::{Db, Tick};

/// Alvo padrão da normalização (dBFS de pico).
pub const DEFAULT_NORMALIZE_TARGET_DBFS: f32 = -1.0;

/// Fades pedidos pela UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FadeRequest {
    pub fade_in_frames: u64,
    pub fade_out_frames: u64,
    pub fade_in_shape: FadeShape,
    pub fade_out_shape: FadeShape,
}

/// Pico absoluto (linear) de uma região `[offset, offset + length)` de todos os canais.
/// Amostras não finitas são ignoradas.
pub fn region_peak(channels: &[Vec<f32>], offset: u64, length: u64) -> f32 {
    let a = offset as usize;
    let b = offset.saturating_add(length) as usize;
    channels
        .iter()
        .flat_map(|c| c.get(a..b.min(c.len())).unwrap_or(&[]))
        .filter(|x| x.is_finite())
        .fold(0.0f32, |m, x| m.max(x.abs()))
}

/// Ganho (dB) que leva o pico `peak` ao alvo `target_dbfs`. `None` para silêncio.
pub fn normalize_gain(peak: f32, target_dbfs: f32) -> Option<Db> {
    (peak.is_finite() && peak > 0.0).then(|| Db(target_dbfs - 20.0 * peak.log10()).clamp_fader())
}

impl Studio {
    /// Corta o clipe pelas bordas (novo início e nova duração). `gesture` agrupa um arraste.
    pub fn clip_trim(
        &mut self,
        id: ClipId,
        start: Tick,
        length_frames: u64,
        gesture: Option<u64>,
    ) -> AppResult<SessionView> {
        self.execute(on_project(TrimClip::new(id, start, length_frames)), gesture)
    }

    /// Divide o clipe no tick `at` (ex.: posição do cursor).
    pub fn clip_split(&mut self, id: ClipId, at: Tick) -> AppResult<SessionView> {
        self.execute(on_project(SplitClip::new(id, at)), None)
    }

    pub fn clip_set_fades(
        &mut self,
        id: ClipId,
        f: FadeRequest,
        gesture: Option<u64>,
    ) -> AppResult<SessionView> {
        self.execute(
            on_project(SetClipFades::new(
                id,
                f.fade_in_frames,
                f.fade_out_frames,
                f.fade_in_shape,
                f.fade_out_shape,
            )),
            gesture,
        )
    }

    pub fn clip_set_polarity(&mut self, id: ClipId, inverted: bool) -> AppResult<SessionView> {
        self.execute(on_project(SetClipPolarity::new(id, inverted)), None)
    }

    pub fn clip_set_reversed(&mut self, id: ClipId, reversed: bool) -> AppResult<SessionView> {
        self.execute(on_project(SetClipReversed::new(id, reversed)), None)
    }

    /// Crossfade entre dois clipes sobrepostos da mesma faixa.
    pub fn clips_crossfade(
        &mut self,
        a: ClipId,
        b: ClipId,
        shape: FadeShape,
    ) -> AppResult<SessionView> {
        self.execute(on_project(Crossfade::new(a, b, shape)), None)
    }

    /// Normaliza o clipe: mede o pico da região referenciada da fonte decodificada e define o
    /// ganho do clipe para levá-lo a `target_dbfs` (−60..=0). Desfazível como "Ganho do clipe".
    pub fn clip_normalize(&mut self, id: ClipId, target_dbfs: f32) -> AppResult<SessionView> {
        if !target_dbfs.is_finite() || !(-60.0..=0.0).contains(&target_dbfs) {
            return Err(AppError::new(
                "api.invalid_argument",
                "Alvo de normalização deve estar entre −60 e 0 dBFS.",
            ));
        }
        let clip = self
            .op()?
            .session
            .project
            .clip(id)
            .map_err(crate::music_support::cmd_err)?
            .clone();
        let buf = self.source_buffer(clip.source_id)?;
        let peak = region_peak(&buf.channels, clip.offset_frames, clip.length_frames);
        let gain = normalize_gain(peak, target_dbfs).ok_or_else(|| {
            AppError::new(
                "edit.silent_clip",
                "O clipe está em silêncio: não há o que normalizar.",
            )
        })?;
        self.execute(on_project(SetClipGain::new(id, gain)), None)
    }

    /// Espectrograma (binário `AMSG`) de uma fonte do projeto, com cache em disco.
    pub fn spectrogram(
        &self,
        source: AudioSourceId,
        params: SpectrogramRequest,
    ) -> AppResult<Vec<u8>> {
        let op = self.op()?;
        let hash = op
            .session
            .project
            .source(source)
            .map_err(crate::music_support::cmd_err)?
            .content_hash
            .clone();
        let buf = self.source_buffer(source)?;
        let sr = self.sample_rate()?;
        Ok(self
            .deps
            .media
            .spectrogram(&buf, sr, params, Some((op.root.as_path(), &hash)))?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn region_peak_reads_only_the_clip_window() {
        let ch = vec![
            vec![0.1, 0.9, 0.2, -0.5, f32::NAN],
            vec![0.0, 0.0, 0.0, -0.7, 0.0],
        ];
        assert_eq!(region_peak(&ch, 2, 3), 0.7);
        assert_eq!(region_peak(&ch, 0, 2), 0.9);
        assert_eq!(region_peak(&ch, 4, 1), 0.0, "NaN ignorado");
        assert_eq!(region_peak(&ch, 10, 5), 0.0, "fora da fonte");
        assert_eq!(region_peak(&ch, u64::MAX, u64::MAX), 0.0, "sem overflow");
    }

    #[test]
    fn normalize_gain_reaches_target_and_is_clamped() {
        let g = normalize_gain(0.5, -1.0).unwrap();
        assert!((g.0 - (-1.0 + 6.0206)).abs() < 1e-3);
        assert_eq!(normalize_gain(0.0, -1.0), None);
        assert_eq!(normalize_gain(f32::NAN, -1.0), None);
        assert_eq!(
            normalize_gain(1e-9, 0.0),
            Some(Db(Db::FADER_MAX)),
            "limitado ao fader"
        );
    }
}
