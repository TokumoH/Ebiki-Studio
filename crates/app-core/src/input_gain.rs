//! Ganho de entrada do microfone/interface e tom de teste da saída — casos de uso da camada de aplicação.
//!
//! Como o "Ouvir entrada" ([`crate::device_monitor`]), são configurações do **dispositivo**, não do
//! projeto: o ganho vale para a sessão do app, é guardado pelo adaptador (app-desktop) junto da
//! escolha de dispositivos e é reaplicado quando a engine é trocada. O tom de teste ("Testar som")
//! sai no dispositivo, fora do master: não entra na gravação nem na exportação.

use crate::{AppError, AppResult, Studio};

/// Faixa aceita (dB): 0 = sinal do dispositivo sem alteração.
pub const INPUT_GAIN_MIN_DB: f64 = audio_engine::INPUT_GAIN_MIN_DB as f64;
pub const INPUT_GAIN_MAX_DB: f64 = audio_engine::INPUT_GAIN_MAX_DB as f64;

/// Tom de teste: duração (s) e nível de pico (dBFS).
pub const TEST_TONE_SECONDS: f32 = 1.0;
pub const TEST_TONE_LEVEL_DBFS: f32 = -12.0;

impl Studio {
    /// Ganho de entrada atual (dB, 0..+24).
    pub fn input_gain_db(&self) -> f64 {
        self.input_gain_db
    }

    /// Define o ganho de entrada (limitado a 0..+24 dB). Vale também sem engine: é aplicado quando
    /// ela for conectada. Valor não finito é recusado.
    pub fn set_input_gain_db(&mut self, db: f64) -> AppResult<f64> {
        if !db.is_finite() {
            return Err(AppError::new("app.invalid", "Ganho de entrada inválido."));
        }
        let db = db.clamp(INPUT_GAIN_MIN_DB, INPUT_GAIN_MAX_DB);
        if let Some(e) = self.engine.as_mut() {
            e.set_input_gain_db(db as f32)
                .map_err(|e| AppError::new("app.engine", e.to_string()))?;
        }
        self.input_gain_db = db;
        Ok(db)
    }

    /// Toca o tom de teste (~1 s, −12 dBFS) na saída do dispositivo.
    pub fn play_test_tone(&mut self) -> AppResult<()> {
        let engine = self
            .engine
            .as_mut()
            .ok_or_else(|| AppError::new("app.no_engine", "Engine de áudio indisponível."))?;
        engine
            .play_test_tone(TEST_TONE_SECONDS, TEST_TONE_LEVEL_DBFS)
            .map_err(|e| AppError::new("app.engine", e.to_string()))
    }

    /// Depois de trocar a engine: reaplica o ganho de entrada.
    pub(crate) fn reapply_input_gain(&mut self) {
        let db = self.input_gain_db as f32;
        if let Some(e) = self.engine.as_mut() {
            // fila cheia logo após criar a engine é impossível; se ocorrer, fica 0 dB até a próxima mudança
            let _ = e.set_input_gain_db(db);
        }
    }
}
