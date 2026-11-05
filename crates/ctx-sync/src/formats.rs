//! Exportação/importação de letra em LRC (linha e "enhanced" por palavra/sílaba) e SRT.
//!
//! Processamento puro de texto (sem I/O): a aplicação lê/grava os arquivos. Tempos convertidos
//! entre ticks e segundos pelo [`TempoMap`] do projeto (BPM variável suportado).
//!
//! Formatos:
//! - LRC de linha: `[mm:ss.xx]texto`.
//! - LRC enhanced: `[mm:ss.xx]<mm:ss.xx>pa<mm:ss.xx>la<mm:ss.xx>vra <mm:ss.xx>outra<mm:ss.xx>`
//!   — uma marca por palavra (granularidade palavra) ou por sílaba; espaço separa palavras;
//!   a marca final fecha a linha.
//! - SRT: um bloco por linha da letra (`HH:MM:SS,mmm --> HH:MM:SS,mmm`).
//!
//! Limites: pausas entre sílabas/palavras de uma mesma linha não são representáveis em LRC
//! (o fim de cada trecho é o início do seguinte); linhas sem tempo não são exportadas;
//! metadados LRC (`[ar:]`, `[ti:]`, `[offset:]`…) são ignorados na importação; a precisão é de
//! 10 ms (LRC) e 1 ms (SRT).

use crate::error::SyncError;
use crate::lyrics::{
    distribute, Granularity, LyricLine, LyricTrack, LyricUnit, LyricWord, Span, Syllable, WordRef,
};
use shared_kernel::tempo::TempoMap;
use shared_kernel::time::Tick;
use std::fmt::Write as _;

/// Duração atribuída à última linha quando o arquivo não informa o fim (segundos).
pub const DEFAULT_LAST_LINE_SECONDS: f64 = 2.0;

fn parse_err(line: usize, reason: &str) -> SyncError {
    SyncError::Parse {
        line,
        reason: reason.to_owned(),
    }
}

// ---------------------------------------------------------------- tempo (texto)

/// `mm:ss.xx` (centésimos arredondados).
pub fn format_lrc_time(seconds: f64) -> String {
    let cs = (seconds.max(0.0) * 100.0).round() as u64;
    format!("{:02}:{:02}.{:02}", cs / 6000, (cs / 100) % 60, cs % 100)
}

/// Lê `mm:ss`, `mm:ss.x`, `mm:ss.xx` ou `mm:ss.xxx`.
pub fn parse_lrc_time(s: &str) -> Option<f64> {
    let (m, rest) = s.trim().split_once(':')?;
    let minutes: u64 = m.parse().ok()?;
    let (sec, frac) = match rest.split_once(['.', ',']) {
        Some((a, b)) => (a, Some(b)),
        None => (rest, None),
    };
    let seconds: u64 = sec.parse().ok()?;
    if seconds >= 60 || !sec.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let fraction = match frac {
        None => 0.0,
        Some(f) if !f.is_empty() && f.len() <= 3 && f.chars().all(|c| c.is_ascii_digit()) => {
            f.parse::<u64>().ok()? as f64 / 10f64.powi(f.len() as i32)
        }
        _ => return None,
    };
    Some(minutes as f64 * 60.0 + seconds as f64 + fraction)
}

/// `HH:MM:SS,mmm`.
pub fn format_srt_time(seconds: f64) -> String {
    let ms = (seconds.max(0.0) * 1000.0).round() as u64;
    format!(
        "{:02}:{:02}:{:02},{:03}",
        ms / 3_600_000,
        (ms / 60_000) % 60,
        (ms / 1000) % 60,
        ms % 1000
    )
}

/// Lê `HH:MM:SS,mmm` (aceita `.` como separador).
pub fn parse_srt_time(s: &str) -> Option<f64> {
    let mut parts = s.trim().split(':');
    let h: u64 = parts.next()?.parse().ok()?;
    let m: u64 = parts.next()?.parse().ok()?;
    let rest = parts.next()?;
    if parts.next().is_some() || m >= 60 {
        return None;
    }
    let (sec, ms) = rest.split_once([',', '.'])?;
    let sec: u64 = sec.parse().ok()?;
    if sec >= 60 || ms.len() != 3 {
        return None;
    }
    let ms: u64 = ms.parse().ok()?;
    Some((h * 3600 + m * 60 + sec) as f64 + ms as f64 / 1000.0)
}

fn secs(tempo: &TempoMap, t: Tick) -> f64 {
    tempo.tick_to_seconds(t)
}

// ---------------------------------------------------------------- LRC

/// Exporta em LRC na granularidade pedida.
pub fn export_lrc(track: &LyricTrack, tempo: &TempoMap, granularity: Granularity) -> String {
    let mut out = String::new();
    for (li, line) in track.lines().iter().enumerate() {
        let Ok(Some(line_span)) = track.unit_span(LyricUnit::Line { line: li }) else {
            continue;
        };
        let _ = write!(out, "[{}]", format_lrc_time(secs(tempo, line_span.start)));
        match granularity {
            Granularity::Line => out.push_str(&line.text()),
            Granularity::Word | Granularity::Syllable => {
                let last = line.words.len() - 1;
                for (wi, word) in line.words.iter().enumerate() {
                    if granularity == Granularity::Word {
                        let span = track
                            .unit_span(LyricUnit::Word(WordRef { line: li, word: wi }))
                            .ok()
                            .flatten();
                        if let Some(s) = span {
                            let _ = write!(out, "<{}>", format_lrc_time(secs(tempo, s.start)));
                        }
                        out.push_str(&word.text());
                    } else {
                        for syl in &word.syllables {
                            if let Some(s) = syl.span {
                                let _ = write!(out, "<{}>", format_lrc_time(secs(tempo, s.start)));
                            }
                            out.push_str(&syl.text);
                        }
                    }
                    if wi < last {
                        out.push(' ');
                    }
                }
                let _ = write!(out, "<{}>", format_lrc_time(secs(tempo, line_span.end)));
            }
        }
        out.push('\n');
    }
    out
}

/// Trecho de uma linha enhanced: (tempo em segundos ou `None` = sem marca, texto).
type Piece = (Option<f64>, String);

/// Divide `<t>texto<t>texto…` em trechos. Texto antes da primeira marca recebe `line_time`.
fn split_enhanced(rest: &str, line_time: f64, lineno: usize) -> Result<Vec<Piece>, SyncError> {
    let mut pieces = Vec::new();
    let mut s = rest;
    let mut current: Option<f64> = Some(line_time);
    loop {
        match s.find('<') {
            None => {
                pieces.push((current, s.to_owned()));
                break;
            }
            Some(i) => {
                if i > 0 {
                    pieces.push((current, s[..i].to_owned()));
                }
                let close = s[i..]
                    .find('>')
                    .ok_or_else(|| parse_err(lineno, "marca '<' sem '>'"))?;
                let t = parse_lrc_time(&s[i + 1..i + close])
                    .ok_or_else(|| parse_err(lineno, "tempo inválido em marca de palavra"))?;
                current = Some(t);
                s = &s[i + close + 1..];
                if s.is_empty() {
                    pieces.push((current, String::new()));
                    break;
                }
            }
        }
    }
    Ok(pieces)
}

/// Palavra lida: sílabas (texto, início em segundos ou `None`).
type ParsedWord = Vec<(String, Option<f64>)>;

fn words_from_pieces(pieces: &[Piece]) -> (Vec<ParsedWord>, Option<f64>) {
    let mut words: Vec<ParsedWord> = Vec::new();
    let mut cur: ParsedWord = Vec::new();
    let mut end = None;
    let flush = |cur: &mut ParsedWord, words: &mut Vec<ParsedWord>| {
        if !cur.is_empty() {
            words.push(std::mem::take(cur));
        }
    };
    for (t, text) in pieces {
        if text.trim().is_empty() {
            if text.is_empty() {
                end = *t;
            } else {
                flush(&mut cur, &mut words);
                end = *t;
            }
            continue;
        }
        end = None;
        if text.starts_with(char::is_whitespace) {
            flush(&mut cur, &mut words);
        }
        for (k, tok) in text.split_whitespace().enumerate() {
            if k > 0 {
                flush(&mut cur, &mut words);
            }
            cur.push((tok.to_owned(), if k == 0 { *t } else { None }));
        }
        if text.ends_with(char::is_whitespace) {
            flush(&mut cur, &mut words);
        }
    }
    flush(&mut cur, &mut words);
    (words, end)
}

struct RawLine {
    lineno: usize,
    time: f64,
    rest: String,
}

/// Importa LRC (linha ou enhanced). Palavras com uma única marca são re-silabadas e o tempo
/// da palavra é dividido entre as sílabas; palavras com várias marcas mantêm as sílabas lidas.
pub fn import_lrc(text: &str, tempo: &TempoMap) -> Result<LyricTrack, SyncError> {
    let mut raw: Vec<RawLine> = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let lineno = i + 1;
        let mut s = line.trim();
        if s.is_empty() {
            continue;
        }
        let mut times = Vec::new();
        let mut metadata = false;
        while let Some(stripped) = s.strip_prefix('[') {
            let close = stripped
                .find(']')
                .ok_or_else(|| parse_err(lineno, "marca '[' sem ']'"))?;
            let inner = &stripped[..close];
            match parse_lrc_time(inner) {
                Some(t) => times.push(t),
                None if inner.split_once(':').is_some_and(|(k, _)| {
                    !k.is_empty() && k.chars().all(|c| c.is_ascii_alphabetic())
                }) =>
                {
                    metadata = true
                }
                None => return Err(parse_err(lineno, "tempo de linha inválido")),
            }
            s = &stripped[close + 1..];
        }
        if times.is_empty() {
            if metadata {
                continue;
            }
            return Err(parse_err(lineno, "linha sem marcação de tempo"));
        }
        for t in times {
            raw.push(RawLine {
                lineno,
                time: t,
                rest: s.to_owned(),
            });
        }
    }
    raw.sort_by(|a, b| a.time.total_cmp(&b.time));

    let mut lines = Vec::new();
    for (idx, r) in raw.iter().enumerate() {
        let next_line = raw.get(idx + 1).map(|n| n.time);
        let line = if r.rest.contains('<') {
            let pieces = split_enhanced(&r.rest, r.time, r.lineno)?;
            let (words, end) = words_from_pieces(&pieces);
            build_enhanced_line(&words, end, next_line, tempo, r.lineno)?
        } else {
            let end = next_line.unwrap_or(r.time + DEFAULT_LAST_LINE_SECONDS);
            build_plain_line(&r.rest, r.time, end, tempo)
        };
        if let Some(l) = line {
            lines.push(l);
        }
    }
    if lines.is_empty() {
        return Err(SyncError::EmptyText);
    }
    LyricTrack::from_lines(lines)
}

fn tick(tempo: &TempoMap, seconds: f64) -> Tick {
    tempo.seconds_to_tick(seconds)
}

/// Linha com um único intervalo dividido entre as sílabas (LRC de linha, SRT).
fn build_plain_line(text: &str, start: f64, end: f64, tempo: &TempoMap) -> Option<LyricLine> {
    let mut line = LyricLine::from_text(text);
    if line.words.is_empty() {
        return None;
    }
    let (a, b) = (tick(tempo, start), tick(tempo, end));
    let syls: Vec<&mut Syllable> = line
        .words
        .iter_mut()
        .flat_map(|w| w.syllables.iter_mut())
        .collect();
    let weights: Vec<usize> = syls.iter().map(|s| s.text.chars().count()).collect();
    if let Ok(parts) = Span::new(a, b).and_then(|sp| distribute(sp, &weights)) {
        for (s, p) in syls.into_iter().zip(parts) {
            s.span = Some(p);
        }
    }
    Some(line)
}

fn build_enhanced_line(
    words: &[ParsedWord],
    end: Option<f64>,
    next_line: Option<f64>,
    tempo: &TempoMap,
    lineno: usize,
) -> Result<Option<LyricLine>, SyncError> {
    if words.is_empty() {
        return Ok(None);
    }
    // Inícios marcados devem crescer estritamente.
    let starts: Vec<f64> = words.iter().flatten().filter_map(|(_, t)| *t).collect();
    if starts.windows(2).any(|w| w[1] <= w[0]) {
        return Err(parse_err(lineno, "marcas de tempo fora de ordem na linha"));
    }
    let Some(&last_start) = starts.last() else {
        return Ok(None);
    };
    let line_end = end
        .filter(|e| *e > last_start)
        .or(next_line.filter(|n| *n > last_start))
        .unwrap_or(last_start + DEFAULT_LAST_LINE_SECONDS);
    // Fim de cada trecho marcado = próximo início marcado (ou fim da linha).
    let mut k = 0usize;
    let mut out_words = Vec::with_capacity(words.len());
    for w in words {
        let timed: Vec<(String, Option<Span>)> = w
            .iter()
            .map(|(text, t)| {
                let span = t.map(|s| {
                    k += 1;
                    let e = starts.get(k).copied().unwrap_or(line_end);
                    Span {
                        start: tick(tempo, s),
                        end: tick(tempo, e),
                    }
                });
                (text.clone(), span)
            })
            .collect();
        let word = if timed.len() == 1 {
            // Palavra inteira com uma marca: re-silaba e divide o tempo.
            let mut word = LyricWord::from_text(&timed[0].0);
            if let Some(span) = timed[0].1 {
                let weights: Vec<usize> = word
                    .syllables
                    .iter()
                    .map(|s| s.text.chars().count())
                    .collect();
                match distribute(span, &weights) {
                    Ok(parts) if span.validate().is_ok() => {
                        for (s, p) in word.syllables.iter_mut().zip(parts) {
                            s.span = Some(p);
                        }
                    }
                    _ => {
                        word = LyricWord {
                            syllables: vec![Syllable {
                                text: timed[0].0.clone(),
                                span: Some(span),
                            }],
                        }
                    }
                }
            }
            word
        } else {
            LyricWord {
                syllables: timed
                    .into_iter()
                    .map(|(text, span)| Syllable { text, span })
                    .collect(),
            }
        };
        out_words.push(word);
    }
    Ok(Some(LyricLine { words: out_words }))
}

// ---------------------------------------------------------------- SRT

/// Exporta em SRT (um bloco por linha com tempo).
pub fn export_srt(track: &LyricTrack, tempo: &TempoMap) -> String {
    let mut out = String::new();
    let mut index = 1;
    for (li, line) in track.lines().iter().enumerate() {
        let Ok(Some(span)) = track.unit_span(LyricUnit::Line { line: li }) else {
            continue;
        };
        let _ = write!(
            out,
            "{index}\n{} --> {}\n{}\n\n",
            format_srt_time(secs(tempo, span.start)),
            format_srt_time(secs(tempo, span.end)),
            line.text()
        );
        index += 1;
    }
    out
}

/// Importa SRT: cada bloco vira uma linha; várias linhas de texto no bloco são unidas por espaço.
pub fn import_srt(text: &str, tempo: &TempoMap) -> Result<LyricTrack, SyncError> {
    let normalized = text.replace("\r\n", "\n");
    let mut cues: Vec<(f64, f64, String)> = Vec::new();
    let mut lineno = 0usize;
    for block in normalized.split("\n\n") {
        let block_lines: Vec<&str> = block.lines().collect();
        let first_line = lineno + 1;
        lineno += block.lines().count() + 1;
        let content: Vec<&str> = block_lines
            .iter()
            .map(|l| l.trim())
            .skip_while(|l| l.is_empty())
            .collect();
        if content.is_empty() {
            continue;
        }
        let mut it = content.iter().peekable();
        if it
            .peek()
            .is_some_and(|l| l.chars().all(|c| c.is_ascii_digit()))
        {
            it.next();
        }
        let timing = it
            .next()
            .ok_or_else(|| parse_err(first_line, "bloco sem tempo"))?;
        let (a, b) = timing
            .split_once("-->")
            .ok_or_else(|| parse_err(first_line, "linha de tempo sem '-->'"))?;
        let start =
            parse_srt_time(a).ok_or_else(|| parse_err(first_line, "tempo inicial inválido"))?;
        let end = parse_srt_time(b).ok_or_else(|| parse_err(first_line, "tempo final inválido"))?;
        if end <= start {
            return Err(parse_err(
                first_line,
                "o fim do bloco deve ser após o início",
            ));
        }
        let text = it.copied().collect::<Vec<_>>().join(" ");
        if !text.trim().is_empty() {
            cues.push((start, end, text));
        }
    }
    cues.sort_by(|x, y| x.0.total_cmp(&y.0));
    let lines: Vec<LyricLine> = cues
        .iter()
        .filter_map(|(s, e, t)| build_plain_line(t, *s, *e, tempo))
        .collect();
    if lines.is_empty() {
        return Err(SyncError::EmptyText);
    }
    LyricTrack::from_lines(lines)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lyrics::SyllableRef;
    use proptest::prelude::*;
    use shared_kernel::time::PPQ;

    /// 120 BPM; 90 BPM a partir do compasso 3; 150 BPM a partir do compasso 5.
    fn variable_tempo() -> TempoMap {
        let mut t = TempoMap::default();
        t.set_tempo(Tick::quarters(8), 90.0).unwrap();
        t.set_tempo(Tick::quarters(16), 150.0).unwrap();
        t
    }

    /// Letra com tempos por sílaba espalhados pelos três trechos de tempo.
    fn timed_track(tempo: &TempoMap) -> LyricTrack {
        let mut t = LyricTrack::from_plain_text(
            "Lua cheia sobre o mar\nSaudade que não passa\nCanta comigo agora",
        )
        .unwrap();
        let units = t.units(Granularity::Syllable);
        // Inícios em centésimos exatos de segundo (representáveis em LRC).
        let mut changes = Vec::new();
        let mut sec = 0.5;
        for (i, u) in units.iter().enumerate() {
            let LyricUnit::Syllable(r) = u else {
                unreachable!()
            };
            let start = tempo.seconds_to_tick(sec);
            sec += 0.37 + 0.11 * (i % 3) as f64;
            let gap = if i % 4 == 3 { 0.25 } else { 0.0 };
            let end = tempo.seconds_to_tick(sec - gap);
            changes.push((*r, Some(Span::new(start, end).unwrap())));
        }
        t.apply_spans(&changes).unwrap();
        t
    }

    #[test]
    fn time_formats() {
        assert_eq!(format_lrc_time(65.234), "01:05.23");
        assert_eq!(format_lrc_time(-3.0), "00:00.00");
        assert_eq!(parse_lrc_time("01:05.23"), Some(65.23));
        assert_eq!(parse_lrc_time("1:05"), Some(65.0));
        assert_eq!(parse_lrc_time("00:01.5"), Some(1.5));
        assert_eq!(parse_lrc_time("00:01.234"), Some(1.234));
        assert_eq!(parse_lrc_time("00:61.00"), None);
        assert_eq!(parse_lrc_time("ar:Artista"), None);
        assert_eq!(parse_lrc_time("00:01.x"), None);
        assert_eq!(format_srt_time(3725.0071), "01:02:05,007");
        assert_eq!(parse_srt_time("01:02:05,007"), Some(3725.007));
        assert_eq!(parse_srt_time("00:00:01.500"), Some(1.5));
        assert_eq!(parse_srt_time("00:00:01,5"), None);
        assert_eq!(parse_srt_time("00:61:01,000"), None);
    }

    #[test]
    fn line_level_lrc_uses_tempo_map() {
        let tempo = variable_tempo();
        let mut t = LyricTrack::from_plain_text("um\ndois").unwrap();
        // Linha 2 no compasso 3 (tick 7680): 4 s a 120 BPM.
        t.set_unit_span(
            LyricUnit::Line { line: 0 },
            Span::new(Tick(0), Tick::quarters(1)).unwrap(),
        )
        .unwrap();
        t.set_unit_span(
            LyricUnit::Line { line: 1 },
            Span::new(Tick::quarters(9), Tick::quarters(10)).unwrap(),
        )
        .unwrap();
        // tick 9 semínimas = 4 s + 1 semínima a 90 BPM (0,6667 s) = 4,67 s
        assert_eq!(
            export_lrc(&t, &tempo, Granularity::Line),
            "[00:00.00]um\n[00:04.67]dois\n"
        );
    }

    #[test]
    fn syllable_lrc_roundtrip_with_variable_tempo() {
        let tempo = variable_tempo();
        let t = timed_track(&tempo);
        let lrc = export_lrc(&t, &tempo, Granularity::Syllable);
        assert!(
            lrc.starts_with("[00:00.50]<00:00.50>Lu<00:00.87>a "),
            "{lrc}"
        );
        let back = import_lrc(&lrc, &tempo).unwrap();
        assert_eq!(back.text(), t.text());
        assert_eq!(export_lrc(&back, &tempo, Granularity::Syllable), lrc);
        // Inícios preservados em até 10 ms.
        for ((_, a), (_, b)) in t.syllables().zip(back.syllables()) {
            let (sa, sb) = (a.span.unwrap(), b.span.unwrap());
            let d = (tempo.tick_to_seconds(sa.start) - tempo.tick_to_seconds(sb.start)).abs();
            assert!(d <= 0.006, "{d}");
        }
    }

    #[test]
    fn word_lrc_roundtrip_resyllabifies() {
        let tempo = variable_tempo();
        let t = timed_track(&tempo);
        let lrc = export_lrc(&t, &tempo, Granularity::Word);
        let back = import_lrc(&lrc, &tempo).unwrap();
        assert_eq!(export_lrc(&back, &tempo, Granularity::Word), lrc);
        // "Saudade" volta com 3 sílabas temporizadas.
        let w = &back.lines()[1].words[0];
        assert_eq!(w.syllables.len(), 3);
        assert!(w.syllables.iter().all(|s| s.span.is_some()));
        assert!(back.validate().is_ok());
    }

    #[test]
    fn srt_roundtrip_with_variable_tempo() {
        let tempo = variable_tempo();
        let t = timed_track(&tempo);
        let srt = export_srt(&t, &tempo);
        assert!(srt.starts_with("1\n00:00:00,500 --> "), "{srt}");
        assert_eq!(srt.matches(" --> ").count(), 3);
        let back = import_srt(&srt, &tempo).unwrap();
        assert_eq!(back.text(), t.text());
        assert_eq!(export_srt(&back, &tempo), srt);
        for li in 0..3 {
            let a = t.unit_span(LyricUnit::Line { line: li }).unwrap().unwrap();
            let b = back
                .unit_span(LyricUnit::Line { line: li })
                .unwrap()
                .unwrap();
            assert!((a.start.0 - b.start.0).abs() <= 2 && (a.end.0 - b.end.0).abs() <= 2);
        }
    }

    #[test]
    fn imports_handwritten_files() {
        let tempo = TempoMap::default();
        let lrc = "[ar:Artista]\n[ti:Título]\n\n[00:12.00][00:30.00]Refrão aqui\n[00:05.00]Primeira <00:06.00>linha<00:07.00>\n";
        let t = import_lrc(lrc, &tempo).unwrap();
        assert_eq!(t.text(), "Primeira linha\nRefrão aqui\nRefrão aqui");
        let s = t.syllable(SyllableRef::new(0, 0, 0)).unwrap().span.unwrap();
        assert_eq!(s.start, tempo.seconds_to_tick(5.0));
        // Última linha sem fim explícito: 2 s.
        let last = t.unit_span(LyricUnit::Line { line: 2 }).unwrap().unwrap();
        assert_eq!(last.end, tempo.seconds_to_tick(32.0));

        let srt = "1\r\n00:00:01,000 --> 00:00:02,500\r\nOlá\r\nmundo\r\n\r\n2\r\n00:00:03,000 --> 00:00:04,000\r\nTchau\r\n";
        let t = import_srt(srt, &tempo).unwrap();
        assert_eq!(t.text(), "Olá mundo\nTchau");
        assert_eq!(
            t.unit_span(LyricUnit::Line { line: 0 }).unwrap().unwrap(),
            Span::new(Tick(PPQ * 2), Tick(PPQ * 5)).unwrap()
        );
    }

    #[test]
    fn parse_errors_carry_line_numbers() {
        let tempo = TempoMap::default();
        let e = import_lrc("[00:01.00]ok\nsem tempo", &tempo).unwrap_err();
        assert_eq!(
            e,
            SyncError::Parse {
                line: 2,
                reason: "linha sem marcação de tempo".into()
            }
        );
        assert_eq!(
            import_lrc("[00:01.00]a<00:0x>b", &tempo)
                .unwrap_err()
                .code(),
            "sync.parse_error"
        );
        assert!(import_lrc("[00:02.00]<00:02.00>a <00:01.00>b", &tempo).is_err());
        assert!(import_lrc("[ar:x]", &tempo).is_err());
        assert!(import_srt("1\n00:00:02,000 --> 00:00:01,000\nx", &tempo).is_err());
        assert!(import_srt("1\nsem seta\nx", &tempo).is_err());
        assert_eq!(import_srt("", &tempo), Err(SyncError::EmptyText));
    }

    proptest! {
        /// Para qualquer mapa de tempo (60–240 BPM, com mudança) e tempos em centésimos,
        /// exportar → importar → exportar (LRC por sílaba) é estável.
        #[test]
        fn lrc_export_is_stable(bpm1 in 60.0f64..240.0, bpm2 in 60.0f64..240.0, change in 1i64..16,
                                durs in prop::collection::vec(5u32..120, 9)) {
            let mut tempo = TempoMap::new(bpm1, 4, 4).unwrap();
            tempo.set_tempo(Tick::quarters(change), bpm2).unwrap();
            let mut t = LyricTrack::from_plain_text("ca sa do\nmar a zul").unwrap();
            let refs: Vec<SyllableRef> = t.syllables().map(|(r, _)| r).collect();
            let mut cs = 10u32;
            let mut changes = Vec::new();
            for (r, d) in refs.iter().zip(&durs) {
                let start = tempo.seconds_to_tick(cs as f64 / 100.0);
                cs += d;
                let end = tempo.seconds_to_tick(cs as f64 / 100.0);
                changes.push((*r, Some(Span::new(start, end).unwrap())));
            }
            t.apply_spans(&changes).unwrap();
            let lrc = export_lrc(&t, &tempo, Granularity::Syllable);
            let back = import_lrc(&lrc, &tempo).unwrap();
            prop_assert_eq!(export_lrc(&back, &tempo, Granularity::Syllable), lrc);
        }
    }
}
