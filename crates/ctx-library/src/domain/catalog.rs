//! Agregado [`BaseCatalog`]: a biblioteca de bases com busca e filtros (RF-4.4).

use super::base::{Base, BaseId, BaseKey, BaseKindTag, GenreCode, Instrument, Tag};
use super::error::LibraryError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Filtros de busca (todos opcionais; combinados com E).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BaseQuery {
    /// Texto livre: nome, tags, gênero e progressão (sem diferenciar maiúsculas e acentos).
    pub text: Option<String>,
    pub bpm_min: Option<f64>,
    pub bpm_max: Option<f64>,
    /// Tom exato (tônica + modo).
    pub key: Option<BaseKey>,
    pub genre: Option<GenreCode>,
    pub instrument: Option<Instrument>,
    /// Todas as tags informadas precisam estar presentes.
    pub tags: BTreeSet<Tag>,
    pub kind: Option<BaseKindTag>,
}

impl BaseQuery {
    pub fn validate(&self) -> Result<(), LibraryError> {
        for b in [self.bpm_min, self.bpm_max].into_iter().flatten() {
            if !b.is_finite() || b < 0.0 {
                return Err(LibraryError::InvalidQuery("BPM inválido".into()));
            }
        }
        if let (Some(lo), Some(hi)) = (self.bpm_min, self.bpm_max) {
            if lo > hi {
                return Err(LibraryError::InvalidQuery(
                    "BPM mínimo maior que o máximo".into(),
                ));
            }
        }
        Ok(())
    }
}

/// Minúsculas sem acentos (busca tolerante em PT-BR).
pub fn fold(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            other => other,
        })
        .collect()
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseCatalog {
    bases: Vec<Base>,
}

impl BaseCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reconstrói a partir de dados persistidos (valida tudo).
    pub fn from_bases(list: Vec<Base>) -> Result<Self, LibraryError> {
        let mut c = Self::new();
        for b in list {
            let at = c.bases.len();
            c.insert(b, at)?;
        }
        Ok(c)
    }

    pub fn bases(&self) -> &[Base] {
        &self.bases
    }

    pub fn get(&self, id: BaseId) -> Option<&Base> {
        self.bases.iter().find(|b| b.id == id)
    }

    fn index_of(&self, id: BaseId) -> Result<usize, LibraryError> {
        self.bases
            .iter()
            .position(|b| b.id == id)
            .ok_or(LibraryError::BaseNotFound)
    }

    fn check_path(&self, b: &Base) -> Result<(), LibraryError> {
        if let Some(a) = b.kind.audio() {
            let clash = self
                .bases
                .iter()
                .any(|o| o.id != b.id && o.kind.audio().is_some_and(|x| x.path == a.path));
            if clash {
                return Err(LibraryError::DuplicatePath);
            }
        }
        Ok(())
    }

    /// Insere (qualquer tipo; a semeadura das bases CC0 usa este método). Retorna o índice.
    pub fn insert(&mut self, b: Base, index: usize) -> Result<usize, LibraryError> {
        b.validate()?;
        if self.get(b.id).is_some() {
            return Err(LibraryError::DuplicateId);
        }
        self.check_path(&b)?;
        let at = index.min(self.bases.len());
        self.bases.insert(at, b);
        Ok(at)
    }

    /// Remove uma base do usuário ou gerada (bases CC0 incluídas são somente leitura).
    pub fn remove(&mut self, id: BaseId) -> Result<(Base, usize), LibraryError> {
        let i = self.index_of(id)?;
        if self.bases[i].kind.tag() == BaseKindTag::BundledCc0 {
            return Err(LibraryError::BundledReadOnly);
        }
        Ok((self.bases.remove(i), i))
    }

    /// Substitui a base de mesmo id (mesmo tipo; CC0 somente leitura). Retorna a anterior.
    pub fn replace(&mut self, updated: Base) -> Result<Base, LibraryError> {
        updated.validate()?;
        let i = self.index_of(updated.id)?;
        let current = &self.bases[i];
        if current.kind.tag() == BaseKindTag::BundledCc0 {
            return Err(LibraryError::BundledReadOnly);
        }
        if current.kind.tag() != updated.kind.tag() {
            return Err(LibraryError::KindChange);
        }
        self.check_path(&updated)?;
        Ok(std::mem::replace(&mut self.bases[i], updated))
    }

    /// Busca com filtros; resultado ordenado por nome (sem acentos) e id.
    pub fn search(&self, q: &BaseQuery) -> Result<Vec<&Base>, LibraryError> {
        q.validate()?;
        let needle = q
            .text
            .as_deref()
            .map(|t| fold(t.trim()))
            .filter(|t| !t.is_empty());
        let mut out: Vec<&Base> = self
            .bases
            .iter()
            .filter(|b| q.bpm_min.is_none_or(|lo| b.bpm >= lo))
            .filter(|b| q.bpm_max.is_none_or(|hi| b.bpm <= hi))
            .filter(|b| q.key.is_none_or(|k| b.key == Some(k)))
            .filter(|b| q.genre.as_ref().is_none_or(|g| b.genre.as_ref() == Some(g)))
            .filter(|b| q.instrument.is_none_or(|i| b.instruments.contains(&i)))
            .filter(|b| q.tags.is_subset(&b.tags))
            .filter(|b| q.kind.is_none_or(|k| b.kind.tag() == k))
            .filter(|b| {
                needle.as_deref().is_none_or(|n| {
                    fold(&b.name).contains(n)
                        || b.tags.iter().any(|t| fold(t.as_str()).contains(n))
                        || b.genre.as_ref().is_some_and(|g| g.as_str().contains(n))
                        || b.progression().is_some_and(|p| fold(p).contains(n))
                })
            })
            .collect();
        out.sort_by(|a, b| fold(&a.name).cmp(&fold(&b.name)).then(a.id.cmp(&b.id)));
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::base::{AudioBase, AudioPath, BaseKind, GenerationRecipe, KeyMode, Meter};
    use crate::domain::generator::StylePattern;
    use shared_kernel::music::PitchClass;

    fn key(t: &str, m: KeyMode) -> BaseKey {
        BaseKey::new(PitchClass::parse(t).unwrap(), m)
    }

    fn audio(name: &str, path: &str, bpm: f64, bundled: bool) -> Base {
        let a = AudioBase {
            path: AudioPath::new(path).unwrap(),
            progression: Some("I V vi IV".into()),
        };
        let kind = if bundled {
            BaseKind::BundledCc0(a)
        } else {
            BaseKind::UserAudio(a)
        };
        Base::new(name, kind, bpm, None, Meter::FOUR_FOUR).unwrap()
    }

    fn generated(name: &str, bpm: f64, k: BaseKey, style: StylePattern) -> Base {
        let meter = Meter::new(style.meter_numerator(), 4).unwrap();
        Base::new(
            name,
            BaseKind::Generated(GenerationRecipe {
                progression: "ii7 V7 Imaj7:8".into(),
                bars: 8,
                style,
                drums: style.default_drums(),
                bass: style.default_bass(),
            }),
            bpm,
            Some(k),
            meter,
        )
        .unwrap()
    }

    fn sample() -> BaseCatalog {
        let mut pop = audio("Pop Ensolarado", "cc0/pop1.wav", 120.0, true);
        pop.genre = Some(GenreCode::new("pop").unwrap());
        pop.instruments = [Instrument::AcousticGuitar, Instrument::Drums].into();
        pop.tags = [Tag::new("verão").unwrap(), Tag::new("alegre").unwrap()].into();
        pop.key = Some(key("G", KeyMode::Major));
        let mut bossa = generated(
            "Bossa da Tarde",
            85.0,
            key("F", KeyMode::Major),
            StylePattern::BossaComping,
        );
        bossa.genre = Some(GenreCode::new("bossa_nova").unwrap());
        bossa.instruments = [Instrument::NylonGuitar, Instrument::Bass].into();
        bossa.tags = [Tag::new("calma").unwrap()].into();
        let mut valsa = generated(
            "Valsa Antiga",
            140.0,
            key("A", KeyMode::Minor),
            StylePattern::Waltz,
        );
        valsa.instruments = [Instrument::Accordion].into();
        valsa.tags = [Tag::new("calma").unwrap(), Tag::new("antiga").unwrap()].into();
        let mut minha = audio("Minha gravação", "user/minha.wav", 100.0, false);
        minha.tags = [Tag::new("alegre").unwrap()].into();
        BaseCatalog::from_bases(vec![pop, bossa, valsa, minha]).unwrap()
    }

    fn names(r: Vec<&Base>) -> Vec<String> {
        r.into_iter().map(|b| b.name.clone()).collect()
    }

    #[test]
    fn search_filters() {
        let c = sample();
        let q = |f: fn(&mut BaseQuery)| {
            let mut q = BaseQuery::default();
            f(&mut q);
            names(c.search(&q).unwrap())
        };
        assert_eq!(q(|_| {}).len(), 4);
        assert_eq!(
            q(|q| {
                q.bpm_min = Some(90.0);
                q.bpm_max = Some(125.0);
            }),
            ["Minha gravação", "Pop Ensolarado"]
        );
        assert_eq!(
            q(|q| q.key = Some(key("A", KeyMode::Minor))),
            ["Valsa Antiga"]
        );
        assert_eq!(
            q(|q| q.genre = Some(GenreCode::new("bossa_nova").unwrap())),
            ["Bossa da Tarde"]
        );
        assert_eq!(
            q(|q| q.instrument = Some(Instrument::Accordion)),
            ["Valsa Antiga"]
        );
        assert_eq!(
            q(|q| q.tags = [Tag::new("CALMA").unwrap()].into()),
            ["Bossa da Tarde", "Valsa Antiga"]
        );
        assert_eq!(
            q(|q| q.tags = [Tag::new("calma").unwrap(), Tag::new("antiga").unwrap()].into()),
            ["Valsa Antiga"]
        );
        assert_eq!(q(|q| q.kind = Some(BaseKindTag::Generated)).len(), 2);
        assert_eq!(q(|q| q.text = Some("GRAVACAO".into())), ["Minha gravação"]);
        assert_eq!(q(|q| q.text = Some("verao".into())), ["Pop Ensolarado"]);
        assert_eq!(
            q(|q| q.text = Some("imaj7".into())).len(),
            2,
            "busca na progressão"
        );
        assert_eq!(q(|q| q.text = Some("bossa".into())), ["Bossa da Tarde"]);
        assert_eq!(q(|q| q.text = Some("   ".into())).len(), 4);
        assert!(q(|q| q.text = Some("inexistente".into())).is_empty());
    }

    #[test]
    fn invalid_queries() {
        let c = sample();
        let q = BaseQuery {
            bpm_min: Some(130.0),
            bpm_max: Some(100.0),
            ..Default::default()
        };
        assert!(matches!(c.search(&q), Err(LibraryError::InvalidQuery(_))));
        let q = BaseQuery {
            bpm_min: Some(f64::NAN),
            ..Default::default()
        };
        assert!(c.search(&q).is_err());
    }

    #[test]
    fn catalog_rules() {
        let mut c = sample();
        let bundled_id = c.bases()[0].id;
        assert_eq!(c.remove(bundled_id), Err(LibraryError::BundledReadOnly));
        let mut edited = c.bases()[0].clone();
        edited.name = "Outro".into();
        assert_eq!(c.replace(edited), Err(LibraryError::BundledReadOnly));
        let dup_path = audio("Cópia", "cc0/pop1.wav", 100.0, false);
        assert_eq!(c.insert(dup_path, 0), Err(LibraryError::DuplicatePath));
        let dup_id = c.bases()[3].clone();
        assert_eq!(c.insert(dup_id, 0), Err(LibraryError::DuplicateId));
        let mut changed_kind = c.bases()[1].clone();
        changed_kind.kind = BaseKind::UserAudio(AudioBase {
            path: AudioPath::new("x.wav").unwrap(),
            progression: None,
        });
        assert_eq!(c.replace(changed_kind), Err(LibraryError::KindChange));
        let ghost = audio("Fantasma", "g.wav", 100.0, false);
        assert_eq!(c.remove(ghost.id), Err(LibraryError::BaseNotFound));
        let mut bad = c.bases()[3].clone();
        bad.bpm = 5.0;
        assert_eq!(c.replace(bad), Err(LibraryError::InvalidBpm));
    }

    #[test]
    fn fold_removes_accents_and_case() {
        assert_eq!(fold("Ação Forró"), "acao forro");
    }
}
