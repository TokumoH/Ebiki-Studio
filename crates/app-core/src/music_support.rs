//! Apoio comum aos casos de uso das Fases 6–8 (Pure Fabrication): acesso ao projeto aberto,
//! leitura de arquivos do usuário com limite, gravação de áudio renderizado como nova fonte do
//! projeto e WAV em memória para audição.

use crate::ports::{ExportFormatKind, ExportSettings};
use crate::studio::{AppError, AppResult, OpenProject, Studio};
use audio_engine::SourceBuffer;
use ctx_project::{AudioSource, RelPath};
use shared_kernel::command::CommandError;
use shared_kernel::id::AudioSourceId;
use std::path::Path;
use std::sync::Arc;

/// Pasta (relativa ao projeto) do áudio gerado pelo app (correção, stretch, partitura).
pub const RENDERED_DIR: &str = "audio/rendered";

/// Fonte de áudio nova, gravada no projeto mas ainda não registrada no agregado.
pub(crate) struct RenderedSource {
    pub source: AudioSource,
    pub buffer: SourceBuffer,
    /// Fonte nova (precisa de `AddSource`) ou reaproveitada (mesmo conteúdo já existia).
    pub is_new: bool,
}

pub(crate) fn no_project() -> AppError {
    AppError::new("app.no_project", "Nenhum projeto aberto.")
}

pub(crate) fn io_err(e: std::io::Error) -> AppError {
    let msg = if e.kind() == std::io::ErrorKind::StorageFull {
        "Disco cheio.".to_owned()
    } else {
        e.to_string()
    };
    AppError::new("app.io", format!("Erro de arquivo: {msg}"))
}

pub(crate) fn cmd_err(e: impl Into<CommandError>) -> AppError {
    AppError::from(e.into())
}

/// Lê um arquivo do usuário recusando arquivos maiores que `max_bytes` (antes de ler).
pub(crate) fn read_limited(path: &Path, max_bytes: u64) -> AppResult<Vec<u8>> {
    let meta = std::fs::metadata(path).map_err(io_err)?;
    if !meta.is_file() {
        return Err(AppError::new(
            "app.not_a_file",
            "O caminho não é um arquivo.",
        ));
    }
    if meta.len() > max_bytes {
        return Err(AppError::new(
            "app.file_too_large",
            format!(
                "Arquivo grande demais (limite: {} MiB).",
                max_bytes / (1024 * 1024)
            ),
        ));
    }
    std::fs::read(path).map_err(io_err)
}

/// Escrita atômica (arquivo temporário ao lado + renomear).
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let tmp = path.with_extension(format!(
        "{}.tmp",
        path.extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default()
    ));
    std::fs::write(&tmp, bytes).map_err(io_err)?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        io_err(e)
    })
}

/// Mistura mono (média dos canais).
pub(crate) fn mono_mix(buf: &SourceBuffer) -> Vec<f32> {
    let n = buf.frames() as usize;
    let ch = buf.channels.len().max(1) as f32;
    (0..n)
        .map(|i| buf.channels.iter().map(|c| c[i]).sum::<f32>() / ch)
        .collect()
}

/// WAV PCM float 32-bit em memória (audição na UI sem passar pelo disco).
pub fn wav_bytes(planes: &[Vec<f32>], sample_rate: u32) -> Vec<u8> {
    let channels = planes.len().max(1) as u16;
    let frames = planes.first().map_or(0, Vec::len);
    let data_len = (frames * channels as usize * 4) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&3u16.to_le_bytes()); // IEEE float
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&(sample_rate * channels as u32 * 4).to_le_bytes());
    out.extend_from_slice(&(channels * 4).to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for i in 0..frames {
        for p in planes {
            let v = p.get(i).copied().unwrap_or(0.0);
            out.extend_from_slice(&(if v.is_finite() { v } else { 0.0 }).to_le_bytes());
        }
    }
    out
}

impl Studio {
    pub(crate) fn op(&self) -> AppResult<&OpenProject> {
        self.open.as_ref().ok_or_else(no_project)
    }

    pub(crate) fn sample_rate(&self) -> AppResult<u32> {
        Ok(self.op()?.session.project.format.sample_rate.hz())
    }

    /// Áudio decodificado de uma fonte do projeto.
    pub(crate) fn source_buffer(&self, id: AudioSourceId) -> AppResult<Arc<SourceBuffer>> {
        self.op()?.sources.get(&id).cloned().ok_or_else(|| {
            AppError::new(
                "app.source_unavailable",
                "Áudio indisponível (arquivo ausente ou corrompido).",
            )
        })
    }

    /// Grava `planes` (taxa do projeto) em `audio/rendered/<uuid>.wav` (WAV float 32) e prepara a
    /// fonte. Conteúdo idêntico a uma fonte existente reaproveita a fonte (sem duplicar).
    pub(crate) fn write_rendered_source(
        &self,
        planes: Vec<Vec<f32>>,
        original_name: &str,
    ) -> AppResult<RenderedSource> {
        let frames = planes.first().map_or(0, Vec::len) as u64;
        if planes.is_empty() || frames == 0 {
            return Err(AppError::new(
                "app.empty_audio",
                "A renderização não produziu áudio.",
            ));
        }
        let op = self.op()?;
        let sr = op.session.project.format.sample_rate.hz();
        let dir = op.root.join(RENDERED_DIR);
        std::fs::create_dir_all(&dir).map_err(io_err)?;
        let file_name = format!("{}.wav", uuid::Uuid::now_v7());
        let file = dir.join(&file_name);
        let settings = ExportSettings {
            format: ExportFormatKind::Wav,
            bit_depth: 32,
            normalize: false,
            ..ExportSettings::default()
        };
        self.deps.media.encode(&file, &settings, sr, &planes)?;
        let hash = self.deps.media.hash_file(&file)?;
        if let Some(existing) = op.session.project.source_by_hash(&hash) {
            let _ = std::fs::remove_file(&file);
            return Ok(RenderedSource {
                source: existing.clone(),
                buffer: SourceBuffer { channels: planes },
                is_new: false,
            });
        }
        let rel = RelPath::new(format!("{RENDERED_DIR}/{file_name}")).map_err(cmd_err)?;
        let name: String = original_name
            .chars()
            .filter(|c| !c.is_control())
            .take(120)
            .collect();
        Ok(RenderedSource {
            source: AudioSource {
                id: AudioSourceId::new(),
                content_hash: hash,
                rel_path: rel,
                original_name: name,
                channels: planes.len() as u16,
                length_frames: frames,
            },
            buffer: SourceBuffer { channels: planes },
            is_new: true,
        })
    }

    /// Disponibiliza o áudio decodificado de uma fonte nova para a engine.
    pub(crate) fn register_buffer(&mut self, id: AudioSourceId, buffer: SourceBuffer) {
        if let Some(op) = self.open.as_mut() {
            op.sources.insert(id, Arc::new(buffer));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_bytes_has_valid_header_and_size() {
        let b = wav_bytes(&[vec![0.5, -0.5], vec![1.0, f32::NAN]], 48_000);
        assert_eq!(&b[0..4], b"RIFF");
        assert_eq!(&b[8..16], b"WAVEfmt ");
        assert_eq!(u16::from_le_bytes([b[20], b[21]]), 3, "float");
        assert_eq!(u16::from_le_bytes([b[22], b[23]]), 2, "canais");
        assert_eq!(b.len(), 44 + 2 * 2 * 4);
        let last = f32::from_le_bytes([b[56], b[57], b[58], b[59]]);
        assert_eq!(last, 0.0, "NaN vira silêncio");
    }
}
