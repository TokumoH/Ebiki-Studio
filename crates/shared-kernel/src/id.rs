//! Identificadores tipados (UUID v7: ordenáveis por criação e gerados no domínio).

/// Define um identificador tipado baseado em UUID v7.
#[macro_export]
macro_rules! define_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub uuid::Uuid);

        impl $name {
            /// Gera um novo identificador.
            pub fn new() -> Self {
                Self(uuid::Uuid::now_v7())
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl std::str::FromStr for $name {
            type Err = uuid::Error;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                uuid::Uuid::parse_str(s).map(Self)
            }
        }
    };
}

define_id!(
    /// Identificador de faixa (compartilhado por Project e Mixing).
    TrackId
);
define_id!(
    /// Identificador de fonte de áudio.
    AudioSourceId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_roundtrip_through_string() {
        let a = TrackId::new();
        let b = TrackId::new();
        assert_ne!(a, b);
        let parsed: TrackId = a.to_string().parse().unwrap();
        assert_eq!(a, parsed);
    }

    #[test]
    fn ids_serialize_as_plain_string() {
        let id = TrackId::new();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, format!("\"{id}\""));
    }
}
