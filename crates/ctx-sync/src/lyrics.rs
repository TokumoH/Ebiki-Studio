//! Letra sincronizada por linha, palavra e sílaba (tempo musical em ticks).
//!
//! O texto da palavra e da linha é **derivado** das sílabas (sem redundância): a palavra é a
//! concatenação das sílabas e a linha junta as palavras com espaço. Os tempos ficam só nas
//! sílabas; palavra e linha têm intervalo derivado (primeiro início → último fim).

use crate::error::SyncError;
use crate::hyphenation::split_syllables_pt;
use serde::{Deserialize, Serialize};
use shared_kernel::define_id;
use shared_kernel::time::Tick;

define_id!(
    /// Identificador de uma trilha de letra.
    LyricTrackId
);

/// Intervalo de tempo `[start, end)` em ticks, `0 ≤ start < end`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Span {
    pub start: Tick,
    pub end: Tick,
}

impl Span {
    pub fn new(start: Tick, end: Tick) -> Result<Self, SyncError> {
        let s = Self { start, end };
        s.validate()?;
        Ok(s)
    }

    pub fn validate(&self) -> Result<(), SyncError> {
        if self.start.0 < 0 || self.start >= self.end {
            Err(SyncError::InvalidSpan)
        } else {
            Ok(())
        }
    }

    pub fn len(&self) -> i64 {
        self.end.0 - self.start.0
    }

    pub fn is_empty(&self) -> bool {
        self.len() <= 0
    }

    pub fn shifted(&self, delta: i64) -> Result<Self, SyncError> {
        Self::new(Tick(self.start.0 + delta), Tick(self.end.0 + delta))
    }
}

/// Sílaba (menor unidade sincronizável).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Syllable {
    pub text: String,
    pub span: Option<Span>,
}

impl Syllable {
    pub fn untimed(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            span: None,
        }
    }
}

/// Palavra: sequência de sílabas (texto derivado).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LyricWord {
    pub syllables: Vec<Syllable>,
}

impl LyricWord {
    /// Palavra com separação silábica automática (português) e sem tempos.
    pub fn from_text(text: &str) -> Self {
        Self {
            syllables: split_syllables_pt(text)
                .into_iter()
                .map(Syllable::untimed)
                .collect(),
        }
    }

    pub fn text(&self) -> String {
        self.syllables.iter().map(|s| s.text.as_str()).collect()
    }
}

/// Linha: sequência de palavras.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LyricLine {
    pub words: Vec<LyricWord>,
}

impl LyricLine {
    /// Linha a partir de texto (palavras separadas por espaço).
    pub fn from_text(text: &str) -> Self {
        Self {
            words: text.split_whitespace().map(LyricWord::from_text).collect(),
        }
    }

    pub fn text(&self) -> String {
        self.words
            .iter()
            .map(LyricWord::text)
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Endereço de uma sílaba (ordem lexicográfica = ordem de leitura).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SyllableRef {
    pub line: usize,
    pub word: usize,
    pub syllable: usize,
}

impl SyllableRef {
    pub const fn new(line: usize, word: usize, syllable: usize) -> Self {
        Self {
            line,
            word,
            syllable,
        }
    }
}

/// Endereço de uma palavra.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WordRef {
    pub line: usize,
    pub word: usize,
}

/// Unidade de sincronização.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum LyricUnit {
    Syllable(SyllableRef),
    Word(WordRef),
    Line { line: usize },
}

/// Alterações de tempo (valor anterior de cada sílaba afetada), usadas para desfazer.
pub type SpanChanges = Vec<(SyllableRef, Option<Span>)>;

/// Trilha de letra (agregado).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricTrack {
    pub id: LyricTrackId,
    lines: Vec<LyricLine>,
}

impl LyricTrack {
    /// Trilha a partir de linhas já montadas (validada).
    pub fn from_lines(lines: Vec<LyricLine>) -> Result<Self, SyncError> {
        let t = Self {
            id: LyricTrackId::new(),
            lines,
        };
        t.validate()?;
        Ok(t)
    }

    /// Importa texto puro: uma linha por linha não vazia, palavras por espaço, sílabas por regra.
    pub fn from_plain_text(text: &str) -> Result<Self, SyncError> {
        let lines: Vec<LyricLine> = text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(LyricLine::from_text)
            .collect();
        if lines.is_empty() {
            return Err(SyncError::EmptyText);
        }
        Self::from_lines(lines)
    }

    pub fn lines(&self) -> &[LyricLine] {
        &self.lines
    }

    pub fn text(&self) -> String {
        self.lines
            .iter()
            .map(LyricLine::text)
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn syllable(&self, r: SyllableRef) -> Result<&Syllable, SyncError> {
        self.lines
            .get(r.line)
            .and_then(|l| l.words.get(r.word))
            .and_then(|w| w.syllables.get(r.syllable))
            .ok_or(SyncError::NotFound)
    }

    fn syllable_mut(&mut self, r: SyllableRef) -> Result<&mut Syllable, SyncError> {
        self.lines
            .get_mut(r.line)
            .and_then(|l| l.words.get_mut(r.word))
            .and_then(|w| w.syllables.get_mut(r.syllable))
            .ok_or(SyncError::NotFound)
    }

    /// Todas as sílabas em ordem de leitura.
    pub fn syllables(&self) -> impl Iterator<Item = (SyllableRef, &Syllable)> + '_ {
        self.lines.iter().enumerate().flat_map(|(li, l)| {
            l.words.iter().enumerate().flat_map(move |(wi, w)| {
                w.syllables
                    .iter()
                    .enumerate()
                    .map(move |(si, s)| (SyllableRef::new(li, wi, si), s))
            })
        })
    }

    /// Sílabas cobertas por uma unidade.
    pub fn unit_syllables(&self, unit: LyricUnit) -> Result<Vec<SyllableRef>, SyncError> {
        let refs: Vec<SyllableRef> = match unit {
            LyricUnit::Syllable(r) => {
                self.syllable(r)?;
                vec![r]
            }
            LyricUnit::Word(w) => {
                let word = self
                    .lines
                    .get(w.line)
                    .and_then(|l| l.words.get(w.word))
                    .ok_or(SyncError::NotFound)?;
                (0..word.syllables.len())
                    .map(|s| SyllableRef::new(w.line, w.word, s))
                    .collect()
            }
            LyricUnit::Line { line } => {
                let l = self.lines.get(line).ok_or(SyncError::NotFound)?;
                l.words
                    .iter()
                    .enumerate()
                    .flat_map(|(wi, w)| {
                        (0..w.syllables.len()).map(move |s| SyllableRef::new(line, wi, s))
                    })
                    .collect()
            }
        };
        Ok(refs)
    }

    /// Unidades de uma granularidade, em ordem de leitura.
    pub fn units(&self, granularity: Granularity) -> Vec<LyricUnit> {
        match granularity {
            Granularity::Syllable => self
                .syllables()
                .map(|(r, _)| LyricUnit::Syllable(r))
                .collect(),
            Granularity::Word => self
                .lines
                .iter()
                .enumerate()
                .flat_map(|(li, l)| {
                    (0..l.words.len())
                        .map(move |wi| LyricUnit::Word(WordRef { line: li, word: wi }))
                })
                .collect(),
            Granularity::Line => (0..self.lines.len())
                .map(|line| LyricUnit::Line { line })
                .collect(),
        }
    }

    /// Intervalo derivado da unidade (primeiro início e último fim entre as sílabas com tempo).
    pub fn unit_span(&self, unit: LyricUnit) -> Result<Option<Span>, SyncError> {
        let spans: Vec<Span> = self
            .unit_syllables(unit)?
            .into_iter()
            .filter_map(|r| self.syllable(r).ok().and_then(|s| s.span))
            .collect();
        Ok(match (spans.first(), spans.last()) {
            (Some(a), Some(b)) => Some(Span {
                start: a.start,
                end: b.end,
            }),
            _ => None,
        })
    }

    /// Aplica um conjunto de tempos de forma atômica: valida a trilha inteira e, em caso de
    /// erro, nada muda. Retorna os valores anteriores; reaplicá-los com `apply_spans` desfaz.
    pub fn apply_spans(
        &mut self,
        changes: &[(SyllableRef, Option<Span>)],
    ) -> Result<SpanChanges, SyncError> {
        for (r, span) in changes {
            self.syllable(*r)?;
            if let Some(s) = span {
                s.validate()?;
            }
        }
        let mut previous = Vec::with_capacity(changes.len());
        for (r, span) in changes {
            let syl = self.syllable_mut(*r)?;
            previous.push((*r, std::mem::replace(&mut syl.span, *span)));
        }
        if let Err(e) = self.validate() {
            for (r, span) in previous.iter().rev() {
                if let Ok(s) = self.syllable_mut(*r) {
                    s.span = *span;
                }
            }
            return Err(e);
        }
        // Ordem inversa: reaplicar a lista devolvida restaura o estado mesmo com repetições.
        previous.reverse();
        Ok(previous)
    }

    /// Define o intervalo de uma unidade; palavras/linhas são divididas entre as sílabas
    /// proporcionalmente ao número de caracteres.
    pub fn set_unit_span(&mut self, unit: LyricUnit, span: Span) -> Result<SpanChanges, SyncError> {
        self.set_unit_spans(&[(unit, span)])
    }

    /// Define vários intervalos de uma vez (atômico; usado pela sincronização por toque).
    pub fn set_unit_spans(
        &mut self,
        assignments: &[(LyricUnit, Span)],
    ) -> Result<SpanChanges, SyncError> {
        let mut changes: SpanChanges = Vec::new();
        for (unit, span) in assignments {
            span.validate()?;
            let refs = self.unit_syllables(*unit)?;
            let weights: Vec<usize> = refs
                .iter()
                .map(|r| self.syllable(*r).map(|s| s.text.chars().count().max(1)))
                .collect::<Result<_, _>>()?;
            let spans = distribute(*span, &weights)?;
            changes.extend(refs.into_iter().zip(spans.into_iter().map(Some)));
        }
        self.apply_spans(&changes)
    }

    /// Remove os tempos de uma unidade.
    pub fn clear_unit_span(&mut self, unit: LyricUnit) -> Result<SpanChanges, SyncError> {
        let changes: SpanChanges = self
            .unit_syllables(unit)?
            .into_iter()
            .map(|r| (r, None))
            .collect();
        self.apply_spans(&changes)
    }

    /// Desloca em `delta` ticks as sílabas com tempo entre `from` e `to` (inclusive).
    pub fn shift(
        &mut self,
        from: SyllableRef,
        to: SyllableRef,
        delta: i64,
    ) -> Result<SpanChanges, SyncError> {
        self.syllable(from)?;
        self.syllable(to)?;
        let (lo, hi) = if from <= to { (from, to) } else { (to, from) };
        let changes: SpanChanges = self
            .syllables()
            .filter(|(r, _)| *r >= lo && *r <= hi)
            .filter_map(|(r, s)| s.span.map(|sp| (r, sp)))
            .map(|(r, sp)| sp.shifted(delta).map(|n| (r, Some(n))))
            .collect::<Result<_, _>>()?;
        self.apply_spans(&changes)
    }

    /// Substitui todo o conteúdo (ex.: nova importação). Retorna as linhas anteriores.
    pub fn replace_lines(&mut self, lines: Vec<LyricLine>) -> Result<Vec<LyricLine>, SyncError> {
        let candidate = Self { id: self.id, lines };
        candidate.validate()?;
        Ok(std::mem::replace(&mut self.lines, candidate.lines))
    }

    /// Invariantes: estrutura não vazia, texto sem quebras e tempos monotônicos sem sobreposição
    /// (na ordem de leitura, cada sílaba com tempo começa no fim da anterior ou depois).
    pub fn validate(&self) -> Result<(), SyncError> {
        let mut last_end: Option<Tick> = None;
        for line in &self.lines {
            if line.words.is_empty() {
                return Err(SyncError::EmptyText);
            }
            for word in &line.words {
                if word.syllables.is_empty() {
                    return Err(SyncError::EmptyText);
                }
                for s in &word.syllables {
                    if s.text.is_empty() || s.text.chars().any(char::is_whitespace) {
                        return Err(SyncError::EmptyText);
                    }
                    if let Some(span) = s.span {
                        span.validate()?;
                        if last_end.is_some_and(|e| span.start < e) {
                            return Err(SyncError::Overlap);
                        }
                        last_end = Some(span.end);
                    }
                }
            }
        }
        Ok(())
    }
}

/// Granularidade de sincronização/exportação.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Granularity {
    Syllable,
    Word,
    Line,
}

/// Divide `span` em intervalos contíguos proporcionais aos pesos (cada um com ≥ 1 tick).
pub fn distribute(span: Span, weights: &[usize]) -> Result<Vec<Span>, SyncError> {
    let n = weights.len();
    if n == 0 {
        return Ok(Vec::new());
    }
    if span.len() < n as i64 {
        return Err(SyncError::SpanTooShort(n));
    }
    let total: f64 = weights.iter().map(|w| (*w).max(1) as f64).sum();
    let mut bounds = Vec::with_capacity(n + 1);
    bounds.push(span.start.0);
    let mut acc = 0.0;
    for (i, w) in weights.iter().enumerate().take(n - 1) {
        acc += (*w).max(1) as f64;
        let ideal = span.start.0 + (span.len() as f64 * acc / total).round() as i64;
        let lo = bounds[i] + 1;
        let hi = span.end.0 - (n - 1 - i) as i64;
        bounds.push(ideal.clamp(lo, hi));
    }
    bounds.push(span.end.0);
    Ok(bounds
        .windows(2)
        .map(|w| Span {
            start: Tick(w[0]),
            end: Tick(w[1]),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn span(a: i64, b: i64) -> Span {
        Span::new(Tick(a), Tick(b)).unwrap()
    }

    fn track() -> LyricTrack {
        LyricTrack::from_plain_text("Can-tar o amor\n\n  Saudade de você  \n").unwrap()
    }

    #[test]
    fn plain_text_import_builds_structure() {
        let t = LyricTrack::from_plain_text("Cantar o amor\n\n  Saudade de você  \n").unwrap();
        assert_eq!(t.lines().len(), 2);
        assert_eq!(t.lines()[0].words.len(), 3);
        let w = &t.lines()[1].words[0];
        assert_eq!(
            w.syllables
                .iter()
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>(),
            ["Sau", "da", "de"]
        );
        assert_eq!(t.text(), "Cantar o amor\nSaudade de você");
        assert_eq!(t.syllables().count(), 2 + 1 + 2 + 3 + 1 + 2);
        assert_eq!(
            LyricTrack::from_plain_text(" \n\n"),
            Err(SyncError::EmptyText)
        );
    }

    #[test]
    fn span_validation() {
        assert_eq!(Span::new(Tick(5), Tick(5)), Err(SyncError::InvalidSpan));
        assert_eq!(Span::new(Tick(-1), Tick(5)), Err(SyncError::InvalidSpan));
        assert_eq!(span(0, 10).shifted(-1), Err(SyncError::InvalidSpan));
        assert_eq!(span(0, 10).shifted(5).unwrap(), span(5, 15));
    }

    #[test]
    fn distribute_is_contiguous_and_proportional() {
        let parts = distribute(span(0, 900), &[1, 2, 6]).unwrap();
        assert_eq!(parts, vec![span(0, 100), span(100, 300), span(300, 900)]);
        let tight = distribute(span(10, 13), &[100, 1, 1]).unwrap();
        assert_eq!(tight, vec![span(10, 11), span(11, 12), span(12, 13)]);
        assert_eq!(
            distribute(span(0, 2), &[1, 1, 1]),
            Err(SyncError::SpanTooShort(3))
        );
    }

    #[test]
    fn set_word_and_line_spans() {
        let mut t = track();
        // "Can-tar" é uma palavra com hífen: Can- | tar
        let line0 = LyricUnit::Line { line: 0 };
        t.set_unit_span(line0, span(0, 1920)).unwrap();
        assert_eq!(t.unit_span(line0).unwrap(), Some(span(0, 1920)));
        let w = LyricUnit::Word(WordRef { line: 1, word: 0 });
        t.set_unit_span(w, span(1920, 2880)).unwrap();
        let s = t.syllable(SyllableRef::new(1, 0, 0)).unwrap().span.unwrap();
        assert_eq!(
            s,
            span(1920, 2331),
            "Sau = 3 de 7 caracteres (960·3/7 ≈ 411)"
        );
        // Sobreposição com a linha anterior é rejeitada sem alterar nada.
        let before = t.clone();
        let err = t
            .set_unit_span(
                LyricUnit::Word(WordRef { line: 1, word: 1 }),
                span(1000, 1100),
            )
            .unwrap_err();
        assert_eq!(err, SyncError::Overlap);
        assert_eq!(t, before);
        assert_eq!(
            t.set_unit_span(LyricUnit::Line { line: 9 }, span(0, 10)),
            Err(SyncError::NotFound)
        );
    }

    #[test]
    fn clear_and_shift() {
        let mut t = track();
        t.set_unit_span(LyricUnit::Line { line: 0 }, span(0, 960))
            .unwrap();
        t.set_unit_span(LyricUnit::Line { line: 1 }, span(960, 1920))
            .unwrap();
        let first = SyllableRef::new(0, 0, 0);
        let last = SyllableRef::new(1, 2, 1);
        // Deslocar só a 2ª linha para trás invadiria a 1ª.
        assert_eq!(
            t.shift(SyllableRef::new(1, 0, 0), last, -10),
            Err(SyncError::Overlap)
        );
        // Deslocar tudo funciona; para antes de zero não.
        t.shift(last, first, 100).unwrap();
        assert_eq!(
            t.unit_span(LyricUnit::Line { line: 0 }).unwrap(),
            Some(span(100, 1060))
        );
        assert_eq!(t.shift(first, last, -101), Err(SyncError::InvalidSpan));
        let prev = t.clear_unit_span(LyricUnit::Line { line: 1 }).unwrap();
        assert_eq!(t.unit_span(LyricUnit::Line { line: 1 }).unwrap(), None);
        t.apply_spans(&prev).unwrap();
        assert_eq!(
            t.unit_span(LyricUnit::Line { line: 1 }).unwrap(),
            Some(span(1060, 2020))
        );
    }

    #[test]
    fn units_by_granularity() {
        let t = track();
        assert_eq!(t.units(Granularity::Line).len(), 2);
        assert_eq!(t.units(Granularity::Word).len(), 6);
        assert_eq!(t.units(Granularity::Syllable).len(), t.syllables().count());
    }

    #[test]
    fn validation_rejects_bad_structure() {
        let mut line = LyricLine::from_text("oi");
        line.words[0].syllables[0].text = "o i".into();
        assert_eq!(
            LyricTrack::from_lines(vec![line]),
            Err(SyncError::EmptyText)
        );
        assert_eq!(
            LyricTrack::from_lines(vec![LyricLine { words: vec![] }]),
            Err(SyncError::EmptyText)
        );
    }

    #[test]
    fn serde_roundtrip() {
        let mut t = track();
        t.set_unit_span(LyricUnit::Line { line: 0 }, span(0, 960))
            .unwrap();
        let json = serde_json::to_string(&t).unwrap();
        let back: LyricTrack = serde_json::from_str(&json).unwrap();
        assert_eq!(back, t);
        let unit = serde_json::to_string(&LyricUnit::Line { line: 2 }).unwrap();
        assert_eq!(unit, r#"{"kind":"line","line":2}"#);
    }

    proptest! {
        /// Após qualquer sequência de deslocamentos (aceitos ou rejeitados), a letra continua
        /// monotônica e sem sobreposição.
        #[test]
        fn timings_stay_monotonic_after_shifts(
            gaps in prop::collection::vec(1i64..500, 12),
            ops in prop::collection::vec((0usize..12, 0usize..12, -800i64..800), 1..30),
        ) {
            let mut t = LyricTrack::from_plain_text("um dois três quatro\ncinco seis sete oito\nnove dez onze doze").unwrap();
            let refs: Vec<SyllableRef> = t.syllables().map(|(r, _)| r).collect();
            let units = t.units(Granularity::Word);
            let mut at = 0i64;
            for (u, g) in units.iter().zip(&gaps) {
                t.set_unit_span(*u, span(at, at + 4 * g)).unwrap();
                at += 4 * g + g / 3;
            }
            for (a, b, delta) in ops {
                let (a, b) = (refs[a % refs.len()], refs[b % refs.len()]);
                let before = t.clone();
                match t.shift(a, b, delta) {
                    Ok(prev) => {
                        prop_assert!(t.validate().is_ok());
                        // desfazer exato
                        let mut undo = t.clone();
                        undo.apply_spans(&prev).unwrap();
                        prop_assert_eq!(undo, before);
                    }
                    Err(_) => prop_assert_eq!(&t, &before),
                }
                let starts: Vec<Tick> = t.syllables().filter_map(|(_, s)| s.span).map(|s| s.start).collect();
                prop_assert!(starts.windows(2).all(|w| w[0] <= w[1]));
            }
        }
    }
}
