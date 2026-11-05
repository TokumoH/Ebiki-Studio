//! Render offline com o mesmo núcleo do tempo real (exportação e simulação).

use crate::core::EngineCore;

/// Renderiza `frames` quadros estéreo a partir da posição atual do núcleo (que deve estar tocando).
/// Retorna (esquerdo, direito).
pub fn render(core: &mut EngineCore, frames: usize, block: usize) -> (Vec<f32>, Vec<f32>) {
    let block = block.clamp(1, crate::MAX_BLOCK);
    let mut l = Vec::with_capacity(frames);
    let mut r = Vec::with_capacity(frames);
    let mut buf = vec![0.0f32; block * 2];
    let mut done = 0;
    while done < frames {
        let n = (frames - done).min(block);
        core.process(&mut buf[..n * 2], 2, &[], 0);
        for f in 0..n {
            l.push(buf[f * 2]);
            r.push(buf[f * 2 + 1]);
        }
        done += n;
    }
    (l, r)
}

/// Renderiza com entrada (simulação de gravação/monitoração). `input` intercalado com `in_channels`.
pub fn render_with_input(
    core: &mut EngineCore,
    input: &[f32],
    in_channels: usize,
    block: usize,
) -> (Vec<f32>, Vec<f32>) {
    let block = block.clamp(1, crate::MAX_BLOCK);
    let frames = input.len() / in_channels.max(1);
    let mut l = Vec::with_capacity(frames);
    let mut r = Vec::with_capacity(frames);
    let mut buf = vec![0.0f32; block * 2];
    let mut done = 0;
    while done < frames {
        let n = (frames - done).min(block);
        core.process(
            &mut buf[..n * 2],
            2,
            &input[done * in_channels..(done + n) * in_channels],
            in_channels,
        );
        for f in 0..n {
            l.push(buf[f * 2]);
            r.push(buf[f * 2 + 1]);
        }
        done += n;
    }
    (l, r)
}
