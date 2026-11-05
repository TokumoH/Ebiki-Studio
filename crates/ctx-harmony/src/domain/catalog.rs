//! Catálogo embutido de progressões "com boa química", por gênero.
//!
//! É a **fonte única** dos dados de semente do `library.db`: a aplicação semeia as tabelas
//! `progression*` a partir daqui na primeira execução e sempre que [`CATALOG_VERSION`] aumentar.

use super::progression::{Genre, Progression, ProgressionKey};
use super::scale::Mode;
use std::collections::BTreeSet;
use std::sync::OnceLock;

/// Versão do catálogo embutido (aumente ao alterar [`SPECS`]: dispara nova semeadura).
pub const CATALOG_VERSION: u32 = 1;

struct Spec {
    slug: &'static str,
    name: &'static str,
    mode: Mode,
    genres: &'static [Genre],
    universal: bool,
    beats_per_bar: u8,
    steps: &'static str,
}

use Genre::*;

const fn s(
    slug: &'static str,
    name: &'static str,
    mode: Mode,
    genres: &'static [Genre],
    universal: bool,
    beats_per_bar: u8,
    steps: &'static str,
) -> Spec {
    Spec {
        slug,
        name,
        mode,
        genres,
        universal,
        beats_per_bar,
        steps,
    }
}

const IONIAN: Mode = Mode::Ionian;
const AEOLIAN: Mode = Mode::Aeolian;

#[rustfmt::skip]
const SPECS: &[Spec] = &[
    // ---- Universais (maior) ----
    s("pop-axis", "Eixo do pop (I–V–vi–IV)", IONIAN, &[Pop, Rock, Sertanejo, Gospel, Ballad], true, 4, "I V vi IV"),
    s("doo-wop", "Anos 50 / doo-wop (I–vi–IV–V)", IONIAN, &[Pop, Ballad, RnbSoul], true, 4, "I vi IV V"),
    s("ii-v-i-major", "ii–V–I maior", IONIAN, &[Jazz, BossaNova, Mpb], true, 4, "ii7 V7 Imaj7:8"),
    s("blues-12-bar", "Blues de 12 compassos", IONIAN, &[Blues, Rock, Jazz], true, 4, "I7 I7 I7 I7 IV7 IV7 I7 I7 V7 IV7 I7 V7"),
    s("three-chord", "Três acordes (I–IV–V–I)", IONIAN, &[Rock, Forro, Sertanejo, Gospel, Pop], true, 4, "I IV V I"),
    s("sensitive-female", "Sensível (vi–IV–I–V)", IONIAN, &[Pop, Edm, Rock, Sertanejo], true, 4, "vi IV I V"),
    s("pachelbel-canon", "Cânone de Pachelbel", IONIAN, &[Ballad, Pop, Gospel], true, 4, "I:2 V/7:2 vi:2 iii/5:2 IV:2 I/3:2 IV:2 V:2"),
    s("turnaround", "Turnaround (I–vi–ii–V)", IONIAN, &[Jazz, Pop, Ballad, Mpb], true, 4, "Imaj7:2 vi7:2 ii7:2 V7:2"),
    s("diatonic-circle", "Ciclo das quintas diatônico", IONIAN, &[Jazz, Pop, Mpb, Gospel], true, 4, "I IV vii° iii vi ii V I"),
    // ---- Pop / Balada ----
    s("pop-i-iv-vi-v", "Pop: I–IV–vi–V", IONIAN, &[Pop, Sertanejo, Gospel], false, 4, "I IV vi V"),
    s("pop-iv-i-v-vi", "Pop: IV–I–V–vi", IONIAN, &[Pop, Edm, Rock], false, 4, "IV I V vi"),
    s("royal-road", "Royal road (IV–V–iii–vi)", IONIAN, &[Pop, Edm, Ballad], false, 4, "IVmaj7 V7 iii7 vi"),
    s("ballad-descending-bass", "Balada com baixo descendente", IONIAN, &[Ballad, Pop, Gospel, Mpb], false, 4, "I V/7 vi I/5 IV I/3 ii V"),
    s("ballad-suspension", "Balada com suspensão", IONIAN, &[Ballad, Pop, Gospel], false, 4, "Imaj7 vi7 IVmaj7 V7sus4:2 V7:2"),
    s("waltz-basic", "Valsa (I–IV–V7–I)", IONIAN, &[Ballad, Mpb], false, 3, "I IV V7 I"),
    // ---- Rock ----
    s("mixolydian-rock", "Rock mixolídio (I–bVII–IV–I)", IONIAN, &[Rock, Pop], false, 4, "I bVII IV I"),
    s("epic-cadence", "Cadência épica (bVI–bVII–I)", IONIAN, &[Rock, Edm, Pop], false, 4, "I bVI bVII I"),
    s("rock-power-biii", "Rock: I–bIII–IV (power chords)", IONIAN, &[Rock], false, 4, "I5 bIII5 IV5 I5"),
    s("rock-minor", "Rock menor (i–VI–VII)", AEOLIAN, &[Rock, Pop], false, 4, "i:8 VI VII"),
    // ---- MPB / Samba / Bossa nova ----
    s("bossa-ipanema", "Bossa: Imaj7–II7–ii7–bII7", IONIAN, &[BossaNova, Jazz, Mpb], false, 4, "Imaj7:8 II7:8 ii7 bII7"),
    s("samba-partido", "Samba: I–VI7–ii7–V7", IONIAN, &[Samba, Mpb, BossaNova, Jazz], false, 4, "I V7/ii ii7 V7"),
    s("samba-minor-subdominant", "Samba com subdominante menor", IONIAN, &[Samba, Mpb, BossaNova], false, 4, "Imaj7 V7/IV IVmaj7 iv6"),
    s("samba-minor", "Samba menor (i–V7/iv–iv–V7)", AEOLIAN, &[Samba, Mpb, BossaNova], false, 4, "i V7/iv iv V7"),
    s("mpb-passing-diminished", "MPB com diminuto de passagem", IONIAN, &[Mpb, BossaNova, Samba], false, 4, "Imaj7 #i°7 ii7 V7"),
    s("mpb-secondary-chain", "MPB: Imaj7–iii7–VI7–ii7–V7", IONIAN, &[Mpb, Samba, Jazz], false, 4, "Imaj7 iii7 V7/ii ii7:2 V7:2"),
    // ---- Jazz / Blues ----
    s("ii-v-i-minor", "ii–V–i menor", AEOLIAN, &[Jazz, BossaNova, Mpb], true, 4, "iiø7 V7 i7:8"),
    s("autumn-cycle", "Ciclo menor (estilo Autumn Leaves)", AEOLIAN, &[Jazz, BossaNova, Mpb], false, 4, "iv7 VII7 IIImaj7 VImaj7 iiø7 V7 i7:8"),
    s("minor-blues", "Blues menor", AEOLIAN, &[Blues, Jazz, Rock], false, 4, "i7 i7 i7 i7 iv7 iv7 i7 i7 VI7 V7 i7 V7"),
    s("blues-8-bar", "Blues de 8 compassos", IONIAN, &[Blues, Gospel, Rock], false, 4, "I7 V7 IV7 IV7 I7 V7 I7 V7"),
    s("gospel-7-3-6-2-5-1", "Gospel 7–3–6–2–5–1", IONIAN, &[Gospel, RnbSoul, Jazz], false, 4, "viiø7:2 V7/vi:2 vi7:2 V7/V:2 ii7:2 V7:2 Imaj7"),
    // ---- Gospel / R&B / Soul ----
    s("gospel-borrowed-iv", "Gospel com iv emprestado", IONIAN, &[Gospel, Ballad, RnbSoul], false, 4, "I V7/IV IV iv I"),
    s("neo-soul-descending", "Neo-soul descendente", IONIAN, &[RnbSoul, Gospel, Jazz], false, 4, "IVmaj7 iii7 ii7 Imaj7"),
    s("rnb-iv-iii7-vi", "R&B: IVmaj7–III7–vi7–v7–I7", IONIAN, &[RnbSoul, Funk, Pop], false, 4, "IVmaj7 III7 vi7 v7:2 I7:2"),
    s("soul-ii-v-iii-vi", "Soul: ii7–V7–iii7–vi7", IONIAN, &[RnbSoul, Gospel, Jazz], false, 4, "ii7 V7 iii7 vi7"),
    // ---- Sertanejo / Forró ----
    s("sertanejo-romantic", "Sertanejo romântico (I–V/7–vi–IV)", IONIAN, &[Sertanejo, Pop, Ballad], false, 4, "I V/7 vi IV"),
    s("sertanejo-raiz", "Sertanejo raiz (I–V7)", IONIAN, &[Sertanejo, Forro], false, 4, "I I V7 V7 V7 V7 I I"),
    s("guarania", "Guarânia (I–V7–IV) em 3/4", IONIAN, &[Sertanejo, Ballad], false, 3, "I V7 V7 I V7/IV IV V7 I"),
    s("baiao-mixolydian", "Baião mixolídio (I7–VII)", Mode::Mixolydian, &[Forro, Mpb], false, 4, "I7 VII I7 IV"),
    s("xote", "Xote (I–IV–V7–I)", IONIAN, &[Forro, Sertanejo], false, 4, "I IV V7 I"),
    s("forro-minor", "Forró menor (i–iv–V7–i)", AEOLIAN, &[Forro, Samba, Mpb], false, 4, "i iv V7 i"),
    s("phrygian-dominant", "Frígio dominante (I–II–I)", Mode::PhrygianDominant, &[Forro, Rock], false, 4, "I:8 II I"),
    // ---- Funk ----
    s("dorian-vamp", "Vamp dórico (i7–IV7)", Mode::Dorian, &[Funk, RnbSoul, Rock], false, 4, "i7:8 IV7:8"),
    s("funk-one-chord", "Funk de um acorde (I7)", Mode::Mixolydian, &[Funk, RnbSoul], false, 4, "I7:16"),
    s("funk-i7-iv7", "Funk: I7–IV7", Mode::Mixolydian, &[Funk, Blues, RnbSoul], false, 4, "I7:8 IV7:8"),
    // ---- Reggae ----
    s("reggae-i-iv", "Reggae: I–IV", IONIAN, &[Reggae, Pop], false, 4, "I IV"),
    s("reggae-i-iv-v-iv", "Reggae: I–IV–V–IV", IONIAN, &[Reggae, Rock], false, 4, "I IV V IV"),
    s("reggae-minor", "Reggae menor (i7–iv7)", AEOLIAN, &[Reggae], false, 4, "i7:8 iv7:8"),
    // ---- EDM / modal ----
    s("epic-minor", "Épico menor (i–VI–III–VII)", AEOLIAN, &[Edm, Pop, Rock], true, 4, "i VI III VII"),
    s("edm-i-vii-vi-vii", "EDM: i–VII–VI–VII", AEOLIAN, &[Edm, Rock], false, 4, "i VII VI VII"),
    s("edm-vi-vii-i", "EDM: VI–VII–i", AEOLIAN, &[Edm, Pop], false, 4, "VI VII i:8"),
    s("lydian-dream", "Lídio onírico (I–II)", Mode::Lydian, &[Pop, Edm, Ballad], false, 4, "Imaj7:8 II:8"),
    s("phrygian-vamp", "Frígio (i–II)", Mode::Phrygian, &[Rock, Edm], false, 4, "i:8 II:8"),
    // ---- Menores universais ----
    s("andalusian", "Cadência andaluza (i–VII–VI–V)", AEOLIAN, &[Rock, Mpb, Pop, Ballad], true, 4, "i VII VI V7"),
    s("harmonic-minor-cadence", "Cadência menor harmônica (i–iv–V7–i)", Mode::HarmonicMinor, &[Ballad, Mpb, Gospel, Pop], true, 4, "i iv V7 i"),
];

fn build(spec: &Spec) -> Progression {
    let steps = Progression::parse_steps(spec.steps, spec.mode, spec.beats_per_bar)
        // Invariante coberta por `every_builtin_progression_is_valid`.
        .unwrap_or_else(|e| panic!("catálogo embutido inválido ({}): {e}", spec.slug));
    Progression {
        key: ProgressionKey::Builtin(spec.slug.to_owned()),
        name: spec.name.to_owned(),
        mode: spec.mode,
        genres: spec.genres.iter().copied().collect::<BTreeSet<_>>(),
        universal: spec.universal,
        beats_per_bar: spec.beats_per_bar,
        steps,
    }
}

/// Progressões embutidas (construídas uma vez, imutáveis).
pub fn builtin_catalog() -> &'static [Progression] {
    static CATALOG: OnceLock<Vec<Progression>> = OnceLock::new();
    CATALOG.get_or_init(|| SPECS.iter().map(build).collect())
}

/// Busca uma progressão embutida pelo slug.
pub fn find_builtin(slug: &str) -> Option<&'static Progression> {
    builtin_catalog()
        .iter()
        .find(|p| matches!(&p.key, ProgressionKey::Builtin(s) if s == slug))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::progression::Notation;
    use crate::domain::scale::Key;
    use shared_kernel::music::PitchClass;
    use std::collections::HashSet;

    #[test]
    fn catalog_is_rich() {
        assert!(builtin_catalog().len() >= 40, "{}", builtin_catalog().len());
        for g in Genre::ALL {
            let n = builtin_catalog()
                .iter()
                .filter(|p| p.genres.contains(&g))
                .count();
            assert!(n >= 3, "gênero {g:?} com apenas {n} progressões");
        }
    }

    #[test]
    fn every_builtin_progression_is_valid() {
        let mut slugs = HashSet::new();
        for p in builtin_catalog() {
            p.validate().unwrap_or_else(|e| panic!("{}: {e}", p.name));
            assert!(!p.genres.is_empty(), "{}", p.name);
            let ProgressionKey::Builtin(slug) = &p.key else {
                panic!("embutida com chave de usuário");
            };
            assert!(slugs.insert(slug.clone()), "slug repetido {slug}");
            assert!(
                slug.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "{slug}"
            );
            for t in 0..12 {
                let key = Key::new(PitchClass::new(t).unwrap(), p.mode);
                p.chords_in(key).unwrap();
            }
            // Ida e volta do texto canônico.
            let again = Progression::parse_steps(&p.steps_text(), p.mode, p.beats_per_bar).unwrap();
            assert_eq!(again, p.steps, "{}", p.name);
        }
    }

    #[test]
    fn known_universals_are_marked() {
        for slug in [
            "pop-axis",
            "ii-v-i-major",
            "blues-12-bar",
            "doo-wop",
            "three-chord",
            "andalusian",
        ] {
            assert!(find_builtin(slug).unwrap().universal, "{slug}");
        }
        assert!(find_builtin("nao-existe").is_none());
    }

    #[test]
    fn spot_check_resolutions() {
        let c = Key::parse("C").unwrap();
        let axis = find_builtin("pop-axis").unwrap();
        assert_eq!(
            axis.format(c, Notation::Cifra).unwrap(),
            ["C", "G", "Am", "F"]
        );
        let blues = find_builtin("blues-12-bar").unwrap();
        assert_eq!(blues.total_beats(), 48);
        assert_eq!(blues.format(c, Notation::Cifra).unwrap()[4], "F7");
        let canon = find_builtin("pachelbel-canon").unwrap();
        assert_eq!(
            canon
                .format(Key::parse("D").unwrap(), Notation::Cifra)
                .unwrap(),
            ["D", "A/C#", "Bm", "F#m/A", "G", "D/F#", "G", "A"]
        );
        let ipanema = find_builtin("bossa-ipanema").unwrap();
        assert_eq!(
            ipanema
                .format(Key::parse("F").unwrap(), Notation::Cifra)
                .unwrap(),
            ["Fmaj7", "G7", "Gm7", "Gb7"]
        );
        let andalusian = find_builtin("andalusian").unwrap();
        assert_eq!(
            andalusian
                .format(Key::parse("Am").unwrap(), Notation::Cifra)
                .unwrap(),
            ["Am", "G", "F", "E7"]
        );
        let guarania = find_builtin("guarania").unwrap();
        assert_eq!(guarania.beats_per_bar, 3);
        assert!(guarania.steps.iter().all(|s| s.beats == 3));
    }
}
