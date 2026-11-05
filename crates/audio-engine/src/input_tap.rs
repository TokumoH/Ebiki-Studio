//! Derivação (tap) de um canal de entrada para análise na thread de controle (afinador).
//!
//! O núcleo copia as amostras do canal escolhido para uma fila SPSC lock-free (`rtrb`) sem alocar;
//! se a fila estiver cheia (leitor lento), as amostras excedentes são descartadas — o afinador só
//! precisa das mais recentes. A thread de controle drena a fila com [`InputTapReader::drain`].

/// Lado tempo real.
pub struct InputTapRt {
    channel: usize,
    producer: rtrb::Producer<f32>,
}

impl InputTapRt {
    /// Copia o canal do bloco intercalado (`frames` quadros). Sem alocação nem bloqueio.
    pub fn capture(&mut self, input: &[f32], in_channels: usize, frames: usize) {
        if in_channels == 0 || self.channel >= in_channels {
            return;
        }
        let n = frames.min(self.producer.slots());
        if n == 0 {
            return;
        }
        let Ok(mut chunk) = self.producer.write_chunk_uninit(n) else {
            return;
        };
        let (a, b) = chunk.as_mut_slices();
        let a_len = a.len();
        for f in 0..n {
            let v = input
                .get(f * in_channels + self.channel)
                .copied()
                .filter(|x| x.is_finite())
                .unwrap_or(0.0);
            if f < a_len {
                a[f].write(v);
            } else {
                b[f - a_len].write(v);
            }
        }
        // SAFETY: todas as `n` posições foram inicializadas acima.
        unsafe { chunk.commit_all() };
    }
}

/// Lado de controle.
pub struct InputTapReader {
    consumer: rtrb::Consumer<f32>,
}

impl InputTapReader {
    /// Move tudo o que está na fila para `out` (anexa). Retorna quantas amostras leu.
    pub fn drain(&mut self, out: &mut Vec<f32>) -> usize {
        let n = self.consumer.slots();
        let Ok(chunk) = self.consumer.read_chunk(n) else {
            return 0;
        };
        let (a, b) = chunk.as_slices();
        out.extend_from_slice(a);
        out.extend_from_slice(b);
        chunk.commit_all();
        n
    }
}

/// Cria o par (tempo real, controle) para o canal `channel`, com fila de `capacity` amostras.
pub fn input_tap(channel: usize, capacity: usize) -> (InputTapRt, InputTapReader) {
    let (producer, consumer) = rtrb::RingBuffer::new(capacity.max(1));
    (
        InputTapRt { channel, producer },
        InputTapReader { consumer },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_one_channel_and_drops_when_full() {
        let (mut rt, mut rd) = input_tap(1, 4);
        // 3 quadros estéreo: canal 1 = 10, 11, 12
        rt.capture(&[0.0, 10.0, 0.0, 11.0, 0.0, 12.0], 2, 3);
        rt.capture(&[0.0, 13.0, 0.0, 14.0], 2, 2);
        let mut out = Vec::new();
        assert_eq!(rd.drain(&mut out), 4);
        assert_eq!(out, vec![10.0, 11.0, 12.0, 13.0], "excedente descartado");
        rt.capture(&[0.0, f32::NAN], 2, 1);
        out.clear();
        rd.drain(&mut out);
        assert_eq!(out, vec![0.0], "não finito vira silêncio");
        let (mut bad, mut r2) = input_tap(5, 4);
        bad.capture(&[1.0, 2.0], 2, 1);
        assert_eq!(r2.drain(&mut out), 0, "canal inexistente é ignorado");
    }
}
