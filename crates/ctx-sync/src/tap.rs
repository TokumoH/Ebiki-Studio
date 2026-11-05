//! Sessão de sincronização por toque ("tap-to-sync").
//!
//! O usuário toca (tecla/clique) no início de cada unidade (sílaba, palavra ou linha) durante
//! a reprodução; cada toque fecha a unidade anterior. Ao final, `finish(fim)` fecha a última.
//! A sessão é pura (recebe ticks já convertidos pela aplicação) e produz atribuições que um
//! comando (`ApplyTapSync`) grava na letra de forma atômica e desfazível.
//!
//! ```text
//! Armed --tap--> Syncing --tap--> Syncing --tap (última)--> AwaitingEnd --finish--> Finished
//!   ^              |  ^                                         |
//!   +--undo_tap----+  +-----------------undo_tap----------------+
//! Armed/Syncing/AwaitingEnd --cancel--> Cancelled ; Syncing --finish--> Finished (parcial)
//! ```

use crate::error::SyncError;
use crate::lyrics::{Granularity, LyricTrack, LyricUnit, Span};
use serde::{Deserialize, Serialize};
use shared_kernel::time::Tick;

/// Estados da sessão.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TapState {
    /// Aguardando o primeiro toque.
    Armed,
    /// Recebendo toques.
    Syncing,
    /// Todas as unidades tocadas; aguardando o fim da última.
    AwaitingEnd,
    Finished,
    Cancelled,
}

/// Sessão de sincronização por toque.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TapSyncSession {
    granularity: Granularity,
    units: Vec<LyricUnit>,
    taps: Vec<Tick>,
    end: Option<Tick>,
    state: TapState,
}

impl TapSyncSession {
    /// Inicia a partir da unidade de índice `from` (na granularidade escolhida).
    pub fn start(
        track: &LyricTrack,
        granularity: Granularity,
        from: usize,
    ) -> Result<Self, SyncError> {
        let units: Vec<LyricUnit> = track.units(granularity).into_iter().skip(from).collect();
        if units.is_empty() {
            return Err(SyncError::NothingToSync);
        }
        Ok(Self {
            granularity,
            units,
            taps: Vec::new(),
            end: None,
            state: TapState::Armed,
        })
    }

    pub fn state(&self) -> TapState {
        self.state
    }

    pub fn granularity(&self) -> Granularity {
        self.granularity
    }

    /// Próxima unidade a receber toque.
    pub fn current_unit(&self) -> Option<LyricUnit> {
        matches!(self.state, TapState::Armed | TapState::Syncing)
            .then(|| self.units.get(self.taps.len()).copied())
            .flatten()
    }

    /// (tocadas, total).
    pub fn progress(&self) -> (usize, usize) {
        (self.taps.len(), self.units.len())
    }

    /// Registra o início da unidade atual em `at` (deve ser posterior ao toque anterior).
    pub fn tap(&mut self, at: Tick) -> Result<(), SyncError> {
        if !matches!(self.state, TapState::Armed | TapState::Syncing) {
            return Err(SyncError::InvalidSessionState);
        }
        if at.0 < 0 || self.taps.last().is_some_and(|l| at <= *l) {
            return Err(SyncError::TapOutOfOrder);
        }
        self.taps.push(at);
        self.state = if self.taps.len() == self.units.len() {
            TapState::AwaitingEnd
        } else {
            TapState::Syncing
        };
        Ok(())
    }

    /// Desfaz o último toque.
    pub fn undo_tap(&mut self) -> Result<(), SyncError> {
        if !matches!(self.state, TapState::Syncing | TapState::AwaitingEnd) {
            return Err(SyncError::InvalidSessionState);
        }
        self.taps.pop();
        self.state = if self.taps.is_empty() {
            TapState::Armed
        } else {
            TapState::Syncing
        };
        Ok(())
    }

    /// Encerra: `end` fecha a última unidade tocada. Pode encerrar parcialmente (as unidades
    /// não tocadas ficam como estão).
    pub fn finish(&mut self, end: Tick) -> Result<Vec<(LyricUnit, Span)>, SyncError> {
        if !matches!(self.state, TapState::Syncing | TapState::AwaitingEnd) {
            return Err(SyncError::InvalidSessionState);
        }
        if self.taps.last().is_some_and(|l| end <= *l) {
            return Err(SyncError::TapOutOfOrder);
        }
        self.end = Some(end);
        self.state = TapState::Finished;
        Ok(self.assignments())
    }

    pub fn cancel(&mut self) -> Result<(), SyncError> {
        if matches!(self.state, TapState::Finished | TapState::Cancelled) {
            return Err(SyncError::InvalidSessionState);
        }
        self.state = TapState::Cancelled;
        Ok(())
    }

    /// Atribuições (unidade → intervalo) de uma sessão encerrada; vazio caso contrário.
    pub fn assignments(&self) -> Vec<(LyricUnit, Span)> {
        let Some(end) = self.end.filter(|_| self.state == TapState::Finished) else {
            return Vec::new();
        };
        self.taps
            .iter()
            .enumerate()
            .map(|(i, start)| {
                let stop = self.taps.get(i + 1).copied().unwrap_or(end);
                (
                    self.units[i],
                    Span {
                        start: *start,
                        end: stop,
                    },
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lyrics::{SyllableRef, WordRef};

    fn track() -> LyricTrack {
        LyricTrack::from_plain_text("Lua cheia\nMar azul").unwrap()
    }

    #[test]
    fn full_word_session_produces_contiguous_spans() {
        let t = track();
        let mut s = TapSyncSession::start(&t, Granularity::Word, 0).unwrap();
        assert_eq!(s.state(), TapState::Armed);
        assert_eq!(
            s.current_unit(),
            Some(LyricUnit::Word(WordRef { line: 0, word: 0 }))
        );
        for (i, at) in [0, 960, 2400, 3360].into_iter().enumerate() {
            s.tap(Tick(at)).unwrap();
            assert_eq!(s.progress(), (i + 1, 4));
        }
        assert_eq!(s.state(), TapState::AwaitingEnd);
        assert_eq!(s.current_unit(), None);
        assert_eq!(s.tap(Tick(4000)), Err(SyncError::InvalidSessionState));
        let a = s.finish(Tick(4320)).unwrap();
        assert_eq!(a.len(), 4);
        assert_eq!(a[1].1, Span::new(Tick(960), Tick(2400)).unwrap());
        assert_eq!(a[3].1, Span::new(Tick(3360), Tick(4320)).unwrap());
        assert_eq!(s.state(), TapState::Finished);
        assert_eq!(s.cancel(), Err(SyncError::InvalidSessionState));
        // Aplica na letra.
        let mut t = t;
        t.set_unit_spans(&a).unwrap();
        assert_eq!(
            t.syllable(SyllableRef::new(1, 1, 0))
                .unwrap()
                .span
                .unwrap()
                .start,
            Tick(3360)
        );
    }

    #[test]
    fn taps_must_increase_and_undo_goes_back() {
        let t = track();
        let mut s = TapSyncSession::start(&t, Granularity::Syllable, 2).unwrap();
        assert_eq!(s.undo_tap(), Err(SyncError::InvalidSessionState));
        s.tap(Tick(100)).unwrap();
        assert_eq!(s.tap(Tick(100)), Err(SyncError::TapOutOfOrder));
        assert_eq!(s.tap(Tick(-5)), Err(SyncError::TapOutOfOrder));
        s.tap(Tick(200)).unwrap();
        s.undo_tap().unwrap();
        assert_eq!(s.progress().0, 1);
        s.undo_tap().unwrap();
        assert_eq!(s.state(), TapState::Armed);
        assert_eq!(s.finish(Tick(10)), Err(SyncError::InvalidSessionState));
        s.tap(Tick(300)).unwrap();
        assert_eq!(s.finish(Tick(300)), Err(SyncError::TapOutOfOrder));
        let partial = s.finish(Tick(400)).unwrap();
        assert_eq!(partial.len(), 1, "encerramento parcial");
        // Começou da sílaba de índice 2: "chei" de "Lua cheia" → (0,1,0)
        assert_eq!(partial[0].0, LyricUnit::Syllable(SyllableRef::new(0, 1, 0)));
    }

    #[test]
    fn cancel_and_empty_start() {
        let t = track();
        let mut s = TapSyncSession::start(&t, Granularity::Line, 0).unwrap();
        s.tap(Tick(0)).unwrap();
        s.cancel().unwrap();
        assert_eq!(s.state(), TapState::Cancelled);
        assert!(s.assignments().is_empty());
        assert_eq!(
            TapSyncSession::start(&t, Granularity::Line, 2),
            Err(SyncError::NothingToSync)
        );
    }
}
