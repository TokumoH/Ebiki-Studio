//! Auxílio de mixagem por regras. A porta [`MixAdvisor`] permite trocar por IA no futuro.

use serde::{Deserialize, Serialize};

/// Medições de um canal (produzidas pela engine/analisador).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChannelAnalysis {
    pub name: String,
    pub peak_db: f32,
    pub rms_db: f32,
    pub clipped_samples: u64,
}

/// Medições do master.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MasterAnalysis {
    pub peak_db: f32,
    pub true_peak_db: f32,
    pub rms_db: f32,
    pub integrated_lufs: f32,
    /// Correlação de fase (−1 a +1).
    pub correlation: f32,
    pub clipped_samples: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MixAnalysis {
    pub master: MasterAnalysis,
    pub channels: Vec<ChannelAnalysis>,
    /// Alvo de loudness (padrão de streaming: −14 LUFS).
    pub target_lufs: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Advice {
    pub severity: Severity,
    pub code: String,
    pub channel: Option<String>,
    pub message: String,
}

/// Porta de aconselhamento de mixagem.
pub trait MixAdvisor {
    fn advise(&self, analysis: &MixAnalysis) -> Vec<Advice>;
}

/// Implementação por regras fixas.
#[derive(Debug, Default, Clone, Copy)]
pub struct RuleBasedMixAdvisor;

const CLIP_DB: f32 = -0.1;
const TRUE_PEAK_MAX: f32 = -1.0;
const LUFS_TOLERANCE: f32 = 1.0;
const QUIET_RMS: f32 = -45.0;
const MIN_CREST: f32 = 6.0;

impl MixAdvisor for RuleBasedMixAdvisor {
    fn advise(&self, a: &MixAnalysis) -> Vec<Advice> {
        let mut out = Vec::new();
        let mut push = |severity, code: &str, channel: Option<&str>, message: String| {
            out.push(Advice {
                severity,
                code: code.into(),
                channel: channel.map(Into::into),
                message,
            })
        };
        let m = &a.master;
        if m.clipped_samples > 0 || m.peak_db >= CLIP_DB {
            push(Severity::Critical, "master.clipping", None, format!(
                "O master está clipando ({} amostras). Reduza o volume das faixas ou use um limitador.", m.clipped_samples
            ));
        }
        if m.true_peak_db > TRUE_PEAK_MAX {
            push(Severity::Warning, "master.true_peak", None, format!(
                "True peak de {:.1} dBTP acima de {TRUE_PEAK_MAX} dBTP; pode distorcer após conversão para MP3/AAC.", m.true_peak_db
            ));
        }
        let target = if a.target_lufs.is_finite() && a.target_lufs < 0.0 {
            a.target_lufs
        } else {
            -14.0
        };
        if m.integrated_lufs.is_finite() && m.integrated_lufs > -70.0 {
            let diff = m.integrated_lufs - target;
            if diff > LUFS_TOLERANCE {
                push(Severity::Warning, "master.too_loud", None, format!(
                    "Loudness de {:.1} LUFS está {:.1} dB acima da meta ({target} LUFS); as plataformas vão abaixar o volume.", m.integrated_lufs, diff
                ));
            } else if diff < -LUFS_TOLERANCE {
                push(Severity::Info, "master.too_quiet", None, format!(
                    "Loudness de {:.1} LUFS está {:.1} dB abaixo da meta ({target} LUFS). Considere o limitador no master.", m.integrated_lufs, -diff
                ));
            }
        }
        if m.correlation < 0.0 {
            push(Severity::Warning, "master.phase", None, format!(
                "Correlação de fase negativa ({:.2}): partes da mix podem sumir em mono. Verifique inversões de fase e abertura estéreo.", m.correlation
            ));
        }
        let crest = m.peak_db - m.rms_db;
        if m.rms_db > -70.0 && crest < MIN_CREST {
            push(
                Severity::Warning,
                "master.overcompressed",
                None,
                format!(
                    "Faixa dinâmica de {crest:.1} dB (pico − RMS) indica compressão excessiva."
                ),
            );
        }
        for c in &a.channels {
            if c.clipped_samples > 0 || c.peak_db >= CLIP_DB {
                push(
                    Severity::Critical,
                    "channel.clipping",
                    Some(&c.name),
                    format!(
                        "\"{}\" está clipando. Reduza o ganho do clipe ou o volume.",
                        c.name
                    ),
                );
            } else if c.rms_db < QUIET_RMS && c.rms_db > -140.0 {
                push(
                    Severity::Info,
                    "channel.quiet",
                    Some(&c.name),
                    format!(
                        "\"{}\" tem sinal muito baixo ({:.0} dB RMS).",
                        c.name, c.rms_db
                    ),
                );
            }
        }
        out.sort_by(|a, b| b.severity.cmp(&a.severity));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn healthy() -> MixAnalysis {
        MixAnalysis {
            master: MasterAnalysis {
                peak_db: -1.5,
                true_peak_db: -1.2,
                rms_db: -12.0,
                integrated_lufs: -14.2,
                correlation: 0.6,
                clipped_samples: 0,
            },
            channels: vec![ChannelAnalysis {
                name: "Voz".into(),
                peak_db: -6.0,
                rms_db: -20.0,
                clipped_samples: 0,
            }],
            target_lufs: -14.0,
        }
    }

    #[test]
    fn healthy_mix_has_no_advice() {
        assert!(RuleBasedMixAdvisor.advise(&healthy()).is_empty());
    }

    #[test]
    fn detects_problems_sorted_by_severity() {
        let mut a = healthy();
        a.master.integrated_lufs = -8.0;
        a.master.peak_db = 0.0;
        a.master.correlation = -0.3;
        a.channels[0].rms_db = -60.0;
        let advice = RuleBasedMixAdvisor.advise(&a);
        let codes: Vec<_> = advice.iter().map(|x| x.code.as_str()).collect();
        assert_eq!(advice[0].severity, Severity::Critical);
        for c in [
            "master.clipping",
            "master.too_loud",
            "master.phase",
            "channel.quiet",
        ] {
            assert!(codes.contains(&c), "faltou {c}: {codes:?}");
        }
    }

    #[test]
    fn silence_does_not_trigger_loudness_or_crest_rules() {
        let mut a = healthy();
        a.master = MasterAnalysis {
            peak_db: -144.0,
            true_peak_db: -144.0,
            rms_db: -144.0,
            integrated_lufs: f32::NEG_INFINITY,
            correlation: 1.0,
            clipped_samples: 0,
        };
        a.channels.clear();
        assert!(RuleBasedMixAdvisor.advise(&a).is_empty());
    }
}
