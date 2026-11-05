//! Comandos desfazíveis do contexto Sync.

use crate::lyrics::{LyricLine, LyricTrack, LyricUnit, Span, SpanChanges, SyllableRef};
use crate::warp::{WarpMap, WarpMarker};
use shared_kernel::command::{Command, CommandError};
use shared_kernel::time::Tick;

type R = Result<(), CommandError>;

fn missing() -> CommandError {
    CommandError::new(
        "internal.revert_without_apply",
        "operação desfeita sem ter sido aplicada",
    )
}

/// Desfaz uma alteração de tempos reaplicando os valores anteriores.
fn restore(track: &mut LyricTrack, previous: &mut Option<SpanChanges>) -> R {
    let prev = previous.take().ok_or_else(missing)?;
    track.apply_spans(&prev)?;
    Ok(())
}

/// Define o intervalo de uma sílaba, palavra ou linha.
pub struct SetUnitSpan {
    pub unit: LyricUnit,
    pub span: Span,
    previous: Option<SpanChanges>,
}

impl SetUnitSpan {
    pub fn new(unit: LyricUnit, span: Span) -> Self {
        Self {
            unit,
            span,
            previous: None,
        }
    }
}

impl Command<LyricTrack> for SetUnitSpan {
    fn apply(&mut self, t: &mut LyricTrack) -> R {
        self.previous = Some(t.set_unit_span(self.unit, self.span)?);
        Ok(())
    }
    fn revert(&mut self, t: &mut LyricTrack) -> R {
        restore(t, &mut self.previous)
    }
    fn label(&self) -> &str {
        "Sincronizar trecho da letra"
    }
}

/// Remove os tempos de uma unidade.
pub struct ClearUnitSpan {
    pub unit: LyricUnit,
    previous: Option<SpanChanges>,
}

impl ClearUnitSpan {
    pub fn new(unit: LyricUnit) -> Self {
        Self {
            unit,
            previous: None,
        }
    }
}

impl Command<LyricTrack> for ClearUnitSpan {
    fn apply(&mut self, t: &mut LyricTrack) -> R {
        self.previous = Some(t.clear_unit_span(self.unit)?);
        Ok(())
    }
    fn revert(&mut self, t: &mut LyricTrack) -> R {
        restore(t, &mut self.previous)
    }
    fn label(&self) -> &str {
        "Limpar tempos da letra"
    }
}

/// Desloca um intervalo de sílabas.
pub struct ShiftLyrics {
    pub from: SyllableRef,
    pub to: SyllableRef,
    pub delta: i64,
    previous: Option<SpanChanges>,
}

impl ShiftLyrics {
    pub fn new(from: SyllableRef, to: SyllableRef, delta: i64) -> Self {
        Self {
            from,
            to,
            delta,
            previous: None,
        }
    }
}

impl Command<LyricTrack> for ShiftLyrics {
    fn apply(&mut self, t: &mut LyricTrack) -> R {
        self.previous = Some(t.shift(self.from, self.to, self.delta)?);
        Ok(())
    }
    fn revert(&mut self, t: &mut LyricTrack) -> R {
        restore(t, &mut self.previous)
    }
    fn label(&self) -> &str {
        "Deslocar letra"
    }
}

/// Grava o resultado de uma sessão de sincronização por toque.
pub struct ApplyTapSync {
    assignments: Vec<(LyricUnit, Span)>,
    previous: Option<SpanChanges>,
}

impl ApplyTapSync {
    pub fn new(assignments: Vec<(LyricUnit, Span)>) -> Self {
        Self {
            assignments,
            previous: None,
        }
    }
}

impl Command<LyricTrack> for ApplyTapSync {
    fn apply(&mut self, t: &mut LyricTrack) -> R {
        self.previous = Some(t.set_unit_spans(&self.assignments)?);
        Ok(())
    }
    fn revert(&mut self, t: &mut LyricTrack) -> R {
        restore(t, &mut self.previous)
    }
    fn label(&self) -> &str {
        "Sincronização por toque"
    }
}

/// Substitui o conteúdo da letra (importação de texto, LRC ou SRT).
pub struct ReplaceLyrics {
    lines: Vec<LyricLine>,
    previous: Option<Vec<LyricLine>>,
}

impl ReplaceLyrics {
    pub fn new(lines: Vec<LyricLine>) -> Self {
        Self {
            lines,
            previous: None,
        }
    }
}

impl Command<LyricTrack> for ReplaceLyrics {
    fn apply(&mut self, t: &mut LyricTrack) -> R {
        self.previous = Some(t.replace_lines(self.lines.clone())?);
        Ok(())
    }
    fn revert(&mut self, t: &mut LyricTrack) -> R {
        t.replace_lines(self.previous.take().ok_or_else(missing)?)?;
        Ok(())
    }
    fn label(&self) -> &str {
        "Importar letra"
    }
}

/// Adiciona um marcador de warp.
pub struct AddWarpMarker {
    pub marker: WarpMarker,
}

impl Command<WarpMap> for AddWarpMarker {
    fn apply(&mut self, w: &mut WarpMap) -> R {
        w.add_marker(self.marker)?;
        Ok(())
    }
    fn revert(&mut self, w: &mut WarpMap) -> R {
        w.remove_marker(self.marker.source_frame)?;
        Ok(())
    }
    fn label(&self) -> &str {
        "Adicionar marcador de warp"
    }
}

/// Remove um marcador de warp.
pub struct RemoveWarpMarker {
    pub source_frame: u64,
    removed: Option<WarpMarker>,
}

impl RemoveWarpMarker {
    pub fn new(source_frame: u64) -> Self {
        Self {
            source_frame,
            removed: None,
        }
    }
}

impl Command<WarpMap> for RemoveWarpMarker {
    fn apply(&mut self, w: &mut WarpMap) -> R {
        self.removed = Some(w.remove_marker(self.source_frame)?);
        Ok(())
    }
    fn revert(&mut self, w: &mut WarpMap) -> R {
        w.add_marker(self.removed.take().ok_or_else(missing)?)?;
        Ok(())
    }
    fn label(&self) -> &str {
        "Remover marcador de warp"
    }
}

/// Move o destino (tick) de um marcador de warp.
pub struct MoveWarpMarker {
    pub source_frame: u64,
    pub target: Tick,
    previous: Option<Tick>,
}

impl MoveWarpMarker {
    pub fn new(source_frame: u64, target: Tick) -> Self {
        Self {
            source_frame,
            target,
            previous: None,
        }
    }
}

impl Command<WarpMap> for MoveWarpMarker {
    fn apply(&mut self, w: &mut WarpMap) -> R {
        self.previous = Some(w.move_marker(self.source_frame, self.target)?);
        Ok(())
    }
    fn revert(&mut self, w: &mut WarpMap) -> R {
        w.move_marker(self.source_frame, self.previous.take().ok_or_else(missing)?)?;
        Ok(())
    }
    fn label(&self) -> &str {
        "Mover marcador de warp"
    }
}

/// Altera o andamento nativo da fonte.
pub struct SetSourceBpm {
    pub bpm: f64,
    previous: Option<f64>,
}

impl SetSourceBpm {
    pub fn new(bpm: f64) -> Self {
        Self {
            bpm,
            previous: None,
        }
    }
}

impl Command<WarpMap> for SetSourceBpm {
    fn apply(&mut self, w: &mut WarpMap) -> R {
        self.previous = Some(w.set_source_bpm(self.bpm)?);
        Ok(())
    }
    fn revert(&mut self, w: &mut WarpMap) -> R {
        w.set_source_bpm(self.previous.take().ok_or_else(missing)?)?;
        Ok(())
    }
    fn label(&self) -> &str {
        "BPM da fonte"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lyrics::{Granularity, WordRef};
    use crate::tap::TapSyncSession;
    use shared_kernel::command::History;

    fn roundtrip<T: Clone + PartialEq + std::fmt::Debug + 'static>(
        target: &mut T,
        cmd: Box<dyn Command<T>>,
    ) {
        let before = target.clone();
        let mut h = History::new();
        h.execute(target, cmd, None).unwrap();
        let after = target.clone();
        assert_ne!(before, after, "o comando deveria mudar o estado");
        h.undo(target).unwrap();
        assert_eq!(*target, before);
        h.redo(target).unwrap();
        assert_eq!(*target, after);
    }

    fn span(a: i64, b: i64) -> Span {
        Span::new(Tick(a), Tick(b)).unwrap()
    }

    fn track() -> LyricTrack {
        LyricTrack::from_plain_text("Lua cheia\nMar azul").unwrap()
    }

    #[test]
    fn lyric_commands_roundtrip() {
        let mut t = track();
        roundtrip(
            &mut t,
            Box::new(SetUnitSpan::new(LyricUnit::Line { line: 0 }, span(0, 960))),
        );
        roundtrip(
            &mut t,
            Box::new(SetUnitSpan::new(
                LyricUnit::Word(WordRef { line: 1, word: 1 }),
                span(2000, 2600),
            )),
        );
        // Reescrever o mesmo trecho e desfazer volta ao valor anterior (não a "sem tempo").
        roundtrip(
            &mut t,
            Box::new(SetUnitSpan::new(LyricUnit::Line { line: 0 }, span(10, 900))),
        );
        roundtrip(
            &mut t,
            Box::new(ShiftLyrics::new(
                SyllableRef::new(0, 0, 0),
                SyllableRef::new(1, 1, 1),
                480,
            )),
        );
        roundtrip(
            &mut t,
            Box::new(ClearUnitSpan::new(LyricUnit::Line { line: 1 })),
        );
        roundtrip(
            &mut t,
            Box::new(ReplaceLyrics::new(vec![LyricLine::from_text("nova letra")])),
        );
    }

    #[test]
    fn tap_sync_command_roundtrip() {
        let mut t = track();
        let mut s = TapSyncSession::start(&t, Granularity::Syllable, 0).unwrap();
        for k in 0..t.syllables().count() {
            s.tap(Tick(k as i64 * 300)).unwrap();
        }
        let a = s.finish(Tick(5000)).unwrap();
        roundtrip(&mut t, Box::new(ApplyTapSync::new(a)));
        assert!(t.syllables().all(|(_, syl)| syl.span.is_some()));
    }

    #[test]
    fn failing_lyric_command_changes_nothing() {
        let mut t = track();
        t.set_unit_span(LyricUnit::Line { line: 1 }, span(1000, 2000))
            .unwrap();
        let before = t.clone();
        let mut h = History::new();
        let err = h
            .execute(
                &mut t,
                Box::new(SetUnitSpan::new(
                    LyricUnit::Line { line: 0 },
                    span(500, 1500),
                )),
                None,
            )
            .unwrap_err();
        assert_eq!(err.code, "sync.overlap");
        assert_eq!(t, before);
        assert!(!h.can_undo());
        let mut c = ShiftLyrics::new(SyllableRef::new(0, 0, 0), SyllableRef::new(0, 0, 0), 1);
        assert!(c.revert(&mut t).is_err(), "desfazer sem aplicar é erro");
    }

    #[test]
    fn warp_commands_roundtrip() {
        let mut w = WarpMap::new(48_000, 100.0).unwrap();
        roundtrip(
            &mut w,
            Box::new(AddWarpMarker {
                marker: WarpMarker {
                    source_frame: 0,
                    target: Tick(0),
                },
            }),
        );
        w.add_marker(WarpMarker {
            source_frame: 28_800,
            target: Tick(960),
        })
        .unwrap();
        roundtrip(&mut w, Box::new(MoveWarpMarker::new(28_800, Tick(1000))));
        roundtrip(&mut w, Box::new(RemoveWarpMarker::new(0)));
        roundtrip(&mut w, Box::new(SetSourceBpm::new(128.0)));
        let mut bad = MoveWarpMarker::new(28_800, Tick(-5));
        assert_eq!(
            bad.apply(&mut w).unwrap_err().code,
            "sync.non_monotonic_markers"
        );
    }
}
