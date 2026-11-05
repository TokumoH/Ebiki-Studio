//! Escritor FLAC em fluxo sobre o `flacenc` (Rust puro).
//!
//! Escreve `fLaC` + STREAMINFO provisório, depois cada quadro de [`BLOCK_SIZE`] amostras assim que
//! fica cheio; em `finish` regrava o STREAMINFO (mesmo tamanho) com total de amostras e MD5.

use flacenc::bitsink::ByteSink;
use flacenc::component::{BitRepr, Stream, StreamInfo};
use flacenc::config;
use flacenc::error::{Verified, Verify};
use flacenc::source::{Context, Fill, FrameBuf};
use std::io::{Seek, SeekFrom, Write};

pub(crate) const BLOCK_SIZE: usize = 4096;

pub(crate) struct FlacWriter<W: Write + Seek> {
    out: W,
    config: Verified<config::Encoder>,
    info: StreamInfo,
    framebuf: FrameBuf,
    context: Context,
    pending: Vec<i32>,
    channels: usize,
    frame_number: usize,
}

fn err(e: impl std::fmt::Debug) -> String {
    format!("{e:?}")
}

fn header_bytes(info: &StreamInfo) -> Result<Vec<u8>, String> {
    let stream = Stream::with_stream_info(info.clone());
    let mut sink = ByteSink::new();
    stream.write(&mut sink).map_err(err)?;
    Ok(sink.as_slice().to_vec())
}

impl<W: Write + Seek> FlacWriter<W> {
    pub(crate) fn new(
        mut out: W,
        sample_rate: u32,
        channels: usize,
        bits: usize,
    ) -> Result<Self, String> {
        let config = config::Encoder::default()
            .into_verified()
            .map_err(|(_, e)| err(e))?;
        let mut info = StreamInfo::new(sample_rate as usize, channels, bits).map_err(err)?;
        info.set_block_sizes(BLOCK_SIZE, BLOCK_SIZE).map_err(err)?;
        out.write_all(&header_bytes(&info)?).map_err(err)?;
        Ok(Self {
            out,
            config,
            info,
            framebuf: FrameBuf::with_size(channels, BLOCK_SIZE).map_err(err)?,
            context: Context::new(bits, channels),
            pending: Vec::with_capacity(BLOCK_SIZE * channels),
            channels,
            frame_number: 0,
        })
    }

    /// Acrescenta amostras intercaladas já quantizadas.
    pub(crate) fn write_interleaved(&mut self, samples: &[i32]) -> Result<(), String> {
        let block = BLOCK_SIZE * self.channels;
        let mut rest = samples;
        while !rest.is_empty() {
            let take = (block - self.pending.len()).min(rest.len());
            self.pending.extend_from_slice(&rest[..take]);
            rest = &rest[take..];
            if self.pending.len() == block {
                self.flush_block()?;
            }
        }
        Ok(())
    }

    fn flush_block(&mut self) -> Result<(), String> {
        if self.pending.is_empty() {
            return Ok(());
        }
        let mut target = (&mut self.framebuf, &mut self.context);
        target.fill_interleaved(&self.pending).map_err(err)?;
        let frame = flacenc::encode_fixed_size_frame(
            &self.config,
            &self.framebuf,
            self.frame_number,
            &self.info,
        )
        .map_err(err)?;
        self.info.update_frame_info(&frame);
        let mut sink = ByteSink::new();
        frame.write(&mut sink).map_err(err)?;
        self.out.write_all(sink.as_slice()).map_err(err)?;
        self.frame_number += 1;
        self.pending.clear();
        Ok(())
    }

    pub(crate) fn finish(mut self) -> Result<W, String> {
        self.flush_block()?;
        self.info.set_md5_digest(&self.context.md5_digest());
        self.info.set_total_samples(self.context.total_samples());
        // Pela especificação, o bloco mínimo exclui o último bloco (que pode ser menor).
        self.info
            .set_block_sizes(BLOCK_SIZE, BLOCK_SIZE)
            .map_err(err)?;
        let header = header_bytes(&self.info)?;
        let end = self.out.stream_position().map_err(err)?;
        self.out.seek(SeekFrom::Start(0)).map_err(err)?;
        self.out.write_all(&header).map_err(err)?;
        self.out.seek(SeekFrom::Start(end)).map_err(err)?;
        self.out.flush().map_err(err)?;
        Ok(self.out)
    }
}
