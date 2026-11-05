//! Leitura/escrita de zip (.mxl, .gp) com proteção contra zip bomb.

use crate::error::{check_size, IoError, MAX_UNCOMPRESSED_BYTES, MAX_ZIP_ENTRIES};
use std::io::{Cursor, Read, Write};

/// Arquivo zip aberto em memória.
pub struct Archive {
    zip: zip::ZipArchive<Cursor<Vec<u8>>>,
}

impl Archive {
    pub fn open(bytes: &[u8]) -> Result<Self, IoError> {
        check_size(bytes)?;
        let zip = zip::ZipArchive::new(Cursor::new(bytes.to_vec())).map_err(IoError::malformed)?;
        if zip.len() > MAX_ZIP_ENTRIES {
            return Err(IoError::TooLarge("entradas no zip"));
        }
        Ok(Self { zip })
    }

    pub fn names(&self) -> Vec<String> {
        self.zip.file_names().map(str::to_owned).collect()
    }

    /// Lê uma entrada, limitando o tamanho descompactado (independente do cabeçalho declarado).
    pub fn read(&mut self, name: &str) -> Result<Vec<u8>, IoError> {
        let file = self
            .zip
            .by_name(name)
            .map_err(|_| IoError::malformed(format!("entrada ausente: {name}")))?;
        if file.size() > MAX_UNCOMPRESSED_BYTES {
            return Err(IoError::TooLarge("entrada descompactada"));
        }
        let mut out = Vec::new();
        file.take(MAX_UNCOMPRESSED_BYTES + 1)
            .read_to_end(&mut out)
            .map_err(IoError::malformed)?;
        if out.len() as u64 > MAX_UNCOMPRESSED_BYTES {
            return Err(IoError::TooLarge("entrada descompactada"));
        }
        Ok(out)
    }
}

/// Cria um zip com as entradas na ordem dada (`stored` = sem compressão).
pub fn write_zip(entries: &[(&str, &[u8], bool)]) -> Result<Vec<u8>, IoError> {
    let mut zw = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data, stored) in entries {
        let method = if *stored {
            zip::CompressionMethod::Stored
        } else {
            zip::CompressionMethod::Deflated
        };
        let opts = zip::write::SimpleFileOptions::default().compression_method(method);
        zw.start_file(*name, opts).map_err(IoError::malformed)?;
        zw.write_all(data).map_err(IoError::malformed)?;
    }
    Ok(zw.finish().map_err(IoError::malformed)?.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_limits() {
        let z = write_zip(&[("a.txt", b"oi", true), ("b/c.xml", b"<x/>", false)]).unwrap();
        let mut a = Archive::open(&z).unwrap();
        assert_eq!(a.names().len(), 2);
        assert_eq!(a.read("b/c.xml").unwrap(), b"<x/>");
        assert!(a.read("nada").is_err());
        assert!(Archive::open(b"PK\x03\x04lixo").is_err());
    }

    #[test]
    fn rejects_bomb_entry() {
        // 65 MiB de zeros comprimem para poucos KB
        let big = vec![0u8; (MAX_UNCOMPRESSED_BYTES + 1024 * 1024) as usize];
        let z = write_zip(&[("bomb.xml", &big, false)]).unwrap();
        assert!(z.len() < 1024 * 1024);
        let mut a = Archive::open(&z).unwrap();
        assert!(matches!(a.read("bomb.xml"), Err(IoError::TooLarge(_))));
    }
}
