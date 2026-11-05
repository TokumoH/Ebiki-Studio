//! Casos de uso de letra sincronizada (Fase 6): texto → sílabas, tempos por unidade, deslocamento,
//! sincronização por toque e LRC/SRT. Tudo desfazível; conversão tick ⇄ segundos pelo mapa de
//! andamento do projeto (ADR-0017).

use crate::music_support::{cmd_err, read_limited, write_atomic};
use crate::session::*;
use crate::studio::{AppError, AppResult, Studio};
use crate::view::SessionView;
use ctx_sync::commands::{ApplyTapSync, ClearUnitSpan, ReplaceLyrics, SetUnitSpan, ShiftLyrics};
use ctx_sync::{
    export_lrc, export_srt, import_lrc, import_srt, Granularity, LyricLine, LyricTrack, LyricUnit,
    Span, SyllableRef, TapSyncSession,
};
use serde::{Deserialize, Serialize};
use shared_kernel::Tick;
use std::path::Path;

/// Limite de leitura de arquivos de letra.
pub const MAX_LYRICS_FILE_BYTES: u64 = 2 * 1024 * 1024;
/// Limite de texto colado.
pub const MAX_LYRICS_TEXT_CHARS: usize = 200_000;

/// Formato de arquivo de letra.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "format")]
pub enum LyricsFileFormat {
    /// LRC; `granularity` linha, palavra ou sílaba (LRC "enhanced").
    Lrc {
        granularity: Granularity,
    },
    Srt,
}

/// Formato pela extensão (`.lrc`/`.srt`).
pub fn lyrics_format_of(path: &Path) -> Option<LyricsFileFormat> {
    match path
        .extension()?
        .to_string_lossy()
        .to_ascii_lowercase()
        .as_str()
    {
        "lrc" => Some(LyricsFileFormat::Lrc {
            granularity: Granularity::Syllable,
        }),
        "srt" => Some(LyricsFileFormat::Srt),
        _ => None,
    }
}

impl Studio {
    fn lyrics(&self) -> AppResult<&LyricTrack> {
        Ok(&self.op()?.session.lyrics)
    }

    /// Substitui a letra por texto puro (uma linha por linha; sílabas por regra PT-BR).
    /// Texto vazio limpa a letra.
    pub fn lyrics_set_text(&mut self, text: &str) -> AppResult<SessionView> {
        if text.chars().count() > MAX_LYRICS_TEXT_CHARS {
            return Err(AppError::new(
                "app.invalid_argument",
                "Letra longa demais (máximo de 200 mil caracteres).",
            ));
        }
        let lines: Vec<LyricLine> = if text.trim().is_empty() {
            Vec::new()
        } else {
            LyricTrack::from_plain_text(text)
                .map_err(cmd_err)?
                .lines()
                .to_vec()
        };
        self.execute(on_lyrics(ReplaceLyrics::new(lines)), None)
    }

    /// Define o intervalo de uma sílaba/palavra/linha (palavras e linhas são divididas pelas
    /// sílabas proporcionalmente ao número de letras). `gesture` agrupa arrastes.
    pub fn lyrics_set_span(
        &mut self,
        unit: LyricUnit,
        span: Span,
        gesture: Option<u64>,
    ) -> AppResult<SessionView> {
        span.validate().map_err(cmd_err)?;
        self.execute(on_lyrics(SetUnitSpan::new(unit, span)), gesture)
    }

    pub fn lyrics_clear_span(&mut self, unit: LyricUnit) -> AppResult<SessionView> {
        self.execute(on_lyrics(ClearUnitSpan::new(unit)), None)
    }

    /// Desloca as sílabas com tempo entre `from` e `to` (inclusive) em `delta` ticks.
    pub fn lyrics_shift(
        &mut self,
        from: SyllableRef,
        to: SyllableRef,
        delta: i64,
        gesture: Option<u64>,
    ) -> AppResult<SessionView> {
        self.execute(on_lyrics(ShiftLyrics::new(from, to, delta)), gesture)
    }

    /// Aplica uma sincronização por toque feita na UI: `taps` são os inícios das unidades a
    /// partir da unidade `from` (na granularidade), `end` fecha a última. Valida com a mesma
    /// máquina de estados do domínio e grava atomicamente (um desfazer).
    pub fn lyrics_apply_tap_sync(
        &mut self,
        granularity: Granularity,
        from: usize,
        taps: &[Tick],
        end: Tick,
    ) -> AppResult<SessionView> {
        let mut session =
            TapSyncSession::start(self.lyrics()?, granularity, from).map_err(cmd_err)?;
        for t in taps {
            session.tap(*t).map_err(cmd_err)?;
        }
        let assignments = session.finish(end).map_err(cmd_err)?;
        self.execute(on_lyrics(ApplyTapSync::new(assignments)), None)
    }

    /// Texto do arquivo de letra (LRC/SRT) com os tempos convertidos pelo andamento do projeto.
    pub fn lyrics_render(&self, format: LyricsFileFormat) -> AppResult<String> {
        let op = self.op()?;
        let tempo = op.session.project.tempo();
        Ok(match format {
            LyricsFileFormat::Lrc { granularity } => {
                export_lrc(&op.session.lyrics, tempo, granularity)
            }
            LyricsFileFormat::Srt => export_srt(&op.session.lyrics, tempo),
        })
    }

    /// Exporta para `path` (escrita atômica, UTF-8).
    pub fn lyrics_export(&self, path: &Path, format: LyricsFileFormat) -> AppResult<()> {
        let text = self.lyrics_render(format)?;
        if text.trim().is_empty() {
            return Err(AppError::new(
                "app.nothing_to_export",
                "A letra não tem trechos sincronizados para exportar.",
            ));
        }
        write_atomic(path, text.as_bytes())
    }

    /// Importa LRC/SRT (pela extensão) substituindo a letra (desfazível).
    pub fn lyrics_import(&mut self, path: &Path) -> AppResult<SessionView> {
        let format = lyrics_format_of(path).ok_or_else(|| {
            AppError::new(
                "app.unsupported_format",
                "Formato de letra não suportado (use .lrc ou .srt).",
            )
        })?;
        let bytes = read_limited(path, MAX_LYRICS_FILE_BYTES)?;
        let text = String::from_utf8_lossy(&bytes);
        let text = text.trim_start_matches('\u{feff}');
        let track = {
            let tempo = self.op()?.session.project.tempo();
            match format {
                LyricsFileFormat::Lrc { .. } => import_lrc(text, tempo),
                LyricsFileFormat::Srt => import_srt(text, tempo),
            }
            .map_err(cmd_err)?
        };
        self.execute(on_lyrics(ReplaceLyrics::new(track.lines().to_vec())), None)
    }
}
