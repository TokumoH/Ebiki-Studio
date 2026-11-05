//! Separação silábica do português por regras (sem dicionário).
//!
//! Regras aplicadas:
//! 1. Uma consoante entre vogais inicia a sílaba seguinte (`ca-sa`).
//! 2. Encontros inseparáveis iniciam sílaba: dígrafos `ch`, `lh`, `nh`, `qu`/`gu` antes de vogal
//!    e grupos consoante + `l`/`r` (`pr`, `bl`, `tr`…): `fi-lho`, `pra-to`, `á-gua`.
//! 3. Duas ou mais consoantes: separa antes da última, ou antes das duas últimas se formam
//!    encontro inseparável (`car-ro`, `nas-cer`, `pers-pec-ti-va`, `ins-tru-men-to`).
//! 4. Vogais: ditongo quando a segunda é `i`/`u` átona (`sau-da-de`, `mui-to`) ou em
//!    `ão`/`ãe`/`õe`; hiato com vogal acentuada (`sa-ú-de`), vogais fortes (`po-e-ta`),
//!    `i`/`u` + vogal (`di-a`) e `i`/`u` seguidos de `r/l/z/m/n` em fim de sílaba ou `nh`
//!    (`ca-ir`, `ra-i-nha`, `a-in-da`, `cons-tru-ir`).
//! 5. Pontuação inicial/final fica na primeira/última sílaba; hífen interno encerra sílaba.
//!
//! **Limites conhecidos** (sem dicionário): hiatos/ditongos que dependem da tonicidade sem
//! acento gráfico podem errar (ex.: `Co-im-bra` em vez de `Coim-bra`; `bo-a` é correto, mas
//! `quei-jo`/`tei-a` dependem da pronúncia); estrangeirismos e siglas não seguem as regras.
//! Invariante garantida: a concatenação das sílabas é exatamente a palavra original.

fn is_vowel(c: char) -> bool {
    matches!(
        c,
        'a' | 'e'
            | 'i'
            | 'o'
            | 'u'
            | 'y'
            | 'á'
            | 'à'
            | 'â'
            | 'ã'
            | 'é'
            | 'ê'
            | 'í'
            | 'ó'
            | 'ô'
            | 'õ'
            | 'ú'
            | 'ü'
    )
}

fn is_accented(c: char) -> bool {
    matches!(c, 'á' | 'à' | 'â' | 'é' | 'ê' | 'í' | 'ó' | 'ô' | 'ú')
}

fn inseparable(a: char, b: char) -> bool {
    matches!((a, b), ('c' | 'l' | 'n', 'h') | ('q' | 'g', 'u' | 'ü'))
        || (matches!(a, 'b' | 'c' | 'd' | 'f' | 'g' | 'k' | 'p' | 't' | 'v')
            && matches!(b, 'l' | 'r'))
}

/// Separa uma palavra em sílabas (ver regras e limites no módulo).
pub fn split_syllables_pt(word: &str) -> Vec<String> {
    let chars: Vec<char> = word.chars().collect();
    if chars.is_empty() {
        return Vec::new();
    }
    // Hífen interno: separa em partes, mantendo o hífen no fim da parte anterior.
    if let Some(pos) = chars
        .iter()
        .enumerate()
        .position(|(i, c)| *c == '-' && i > 0 && i + 1 < chars.len())
    {
        let (head, tail): (String, String) = (
            chars[..=pos].iter().collect(),
            chars[pos + 1..].iter().collect(),
        );
        let mut out = split_syllables_pt(&head);
        out.extend(split_syllables_pt(&tail));
        return out;
    }
    let lower: Vec<char> = chars
        .iter()
        .map(|c| c.to_lowercase().next().unwrap_or(*c))
        .collect();
    let n = lower.len();
    // `u` de `qu`/`gu` antes de vogal funciona como consoante.
    let vowel: Vec<bool> = (0..n)
        .map(|i| {
            is_vowel(lower[i])
                && !(matches!(lower[i], 'u' | 'ü')
                    && i > 0
                    && matches!(lower[i - 1], 'q' | 'g')
                    && i + 1 < n
                    && is_vowel(lower[i + 1]))
        })
        .collect();
    let letter = |i: usize| lower[i].is_alphabetic();

    let mut boundaries: Vec<usize> = Vec::new();
    let Some(mut i) = (0..n).find(|&k| vowel[k]) else {
        return vec![word.to_owned()];
    };
    loop {
        // Sequência de vogais a partir de i.
        let mut j = i;
        while j + 1 < n && vowel[j + 1] {
            if !diphthong(&lower, &vowel, j) {
                boundaries.push(j + 1);
            }
            j += 1;
        }
        let Some(k) = (j + 1..n).find(|&k| vowel[k]) else {
            break;
        };
        let cons: Vec<usize> = (j + 1..k).filter(|&c| letter(c)).collect();
        let b = match cons.len() {
            0 | 1 => j + 1,
            len => {
                let (a, z) = (lower[cons[len - 2]], lower[cons[len - 1]]);
                if inseparable(a, z) {
                    cons[len - 2]
                } else {
                    cons[len - 1]
                }
            }
        };
        boundaries.push(b);
        i = k;
    }
    let mut out = Vec::with_capacity(boundaries.len() + 1);
    let mut start = 0;
    for b in boundaries {
        if b > start {
            out.push(chars[start..b].iter().collect());
            start = b;
        }
    }
    out.push(chars[start..].iter().collect());
    out
}

/// As vogais em `j` e `j+1` formam ditongo?
fn diphthong(lower: &[char], vowel: &[bool], j: usize) -> bool {
    let (v1, v2) = (lower[j], lower[j + 1]);
    if matches!((v1, v2), ('ã', 'o' | 'e') | ('õ', 'e')) {
        return true;
    }
    if is_accented(v2) || v1 == v2 || !matches!(v2, 'i' | 'u') {
        return false;
    }
    // i/u seguido de r, l, z, m, n que fecham a sílaba (ou de "nh") → hiato.
    let c1 = lower.get(j + 2).copied();
    let c2 = lower.get(j + 3).copied();
    let closes = |c: char| matches!(c, 'r' | 'l' | 'z' | 'm' | 'n');
    if let Some(c1) = c1.filter(|c| closes(*c)) {
        let next_is_vowel = vowel.get(j + 3).copied().unwrap_or(false);
        let hiatus = match c2 {
            None => true,
            Some(c2) => !next_is_vowel && c2 != c1 && c2.is_alphabetic(),
        };
        if hiatus {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(w: &str) -> String {
        split_syllables_pt(w).join("-")
    }

    #[test]
    fn consonant_rules() {
        assert_eq!(s("casa"), "ca-sa");
        assert_eq!(s("carro"), "car-ro");
        assert_eq!(s("chave"), "cha-ve");
        assert_eq!(s("filho"), "fi-lho");
        assert_eq!(s("ninho"), "ni-nho");
        assert_eq!(s("prato"), "pra-to");
        assert_eq!(s("abraço"), "a-bra-ço");
        assert_eq!(s("pássaro"), "pás-sa-ro");
        assert_eq!(s("nascer"), "nas-cer");
        assert_eq!(s("perspectiva"), "pers-pec-ti-va");
        assert_eq!(s("instrumento"), "ins-tru-men-to");
        assert_eq!(s("transporte"), "trans-por-te");
        assert_eq!(s("ritmo"), "rit-mo");
        assert_eq!(s("advogado"), "ad-vo-ga-do");
        assert_eq!(s("quando"), "quan-do");
        assert_eq!(s("guerra"), "guer-ra");
        assert_eq!(s("água"), "á-gua");
        assert_eq!(s("enquanto"), "en-quan-to");
        assert_eq!(s("coração"), "co-ra-ção");
    }

    #[test]
    fn vowel_rules() {
        assert_eq!(s("saudade"), "sau-da-de");
        assert_eq!(s("muito"), "mui-to");
        assert_eq!(s("poeta"), "po-e-ta");
        assert_eq!(s("saúde"), "sa-ú-de");
        assert_eq!(s("dia"), "di-a");
        assert_eq!(s("rainha"), "ra-i-nha");
        assert_eq!(s("ainda"), "a-in-da");
        assert_eq!(s("construir"), "cons-tru-ir");
        assert_eq!(s("cair"), "ca-ir");
        assert_eq!(s("bairro"), "bair-ro");
        assert_eq!(s("Paulo"), "Pau-lo");
        assert_eq!(s("mãe"), "mãe");
        assert_eq!(s("canções"), "can-ções");
        assert_eq!(s("oceano"), "o-ce-a-no");
        assert_eq!(s("Europa"), "Eu-ro-pa");
    }

    #[test]
    fn punctuation_case_and_edge_cases() {
        assert_eq!(s("Amor,"), "A-mor,");
        assert_eq!(s("\"Vida!\""), "\"Vi-da!\"");
        assert_eq!(s("guarda-chuva"), "guar-da--chu-va");
        assert_eq!(split_syllables_pt("guarda-chuva").len(), 4);
        assert_eq!(s("pneu"), "pneu");
        assert_eq!(s("sol"), "sol");
        assert_eq!(s("123"), "123");
        assert_eq!(s("TCHAU"), "TCHAU");
        assert!(split_syllables_pt("").is_empty());
    }

    #[test]
    fn concatenation_is_lossless() {
        for w in [
            "Ação!",
            "pássaros...",
            "x-y",
            "--",
            "a",
            "ÁGUA",
            "ü",
            "qüinqüênio",
            "o'clock",
        ] {
            assert_eq!(split_syllables_pt(w).concat(), w, "{w}");
        }
    }
}
