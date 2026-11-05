//! Mapeamento de efeitos para tabelas normalizadas (effect_slot / effect_param / eq_band).

use crate::PersistenceError;
use ctx_mixing::{Effect, EffectSlot};
use rusqlite::{params, Connection, Transaction};
use serde_json::{Map, Value};

/// Dono do efeito.
#[derive(Clone, Copy)]
pub enum Owner<'a> {
    Track(&'a str),
    Bus(&'a str),
    Master,
}

pub fn save_slots(
    tx: &Transaction,
    owner: Owner,
    slots: &[EffectSlot],
) -> Result<(), PersistenceError> {
    for (pos, slot) in slots.iter().enumerate() {
        let v = serde_json::to_value(&slot.effect)
            .map_err(|e| PersistenceError::Corrupt(e.to_string()))?;
        let ty = v["type"]
            .as_str()
            .ok_or_else(|| PersistenceError::Corrupt("efeito sem tipo".into()))?
            .to_owned();
        let (track, bus, master) = match owner {
            Owner::Track(t) => (Some(t), None, 0),
            Owner::Bus(b) => (None, Some(b), 0),
            Owner::Master => (None, None, 1),
        };
        let id = slot.id.to_string();
        tx.execute(
            "INSERT INTO effect_slot (id, track_id, bus_id, on_master, position, effect_type, bypass) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, track, bus, master, pos as i64, ty, slot.bypass],
        )?;
        let params_obj = v["params"].as_object().cloned().unwrap_or_default();
        for (name, val) in params_obj {
            match (&slot.effect, name.as_str(), val) {
                (Effect::Eq(_), "bands", Value::Array(bands)) => {
                    for (bp, b) in bands.iter().enumerate() {
                        tx.execute(
                            "INSERT INTO eq_band (effect_id, position, kind, enabled, freq_hz, gain_db, q) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                            params![
                                id,
                                bp as i64,
                                b["kind"].as_str().unwrap_or("peak"),
                                b["enabled"].as_bool().unwrap_or(true),
                                b["freqHz"].as_f64().unwrap_or(1000.0),
                                b["gainDb"].as_f64().unwrap_or(0.0),
                                b["q"].as_f64().unwrap_or(0.707)
                            ],
                        )?;
                    }
                }
                (_, _, Value::Bool(bv)) => {
                    tx.execute(
                        "INSERT INTO effect_param (effect_id, name, kind, value) VALUES (?1, ?2, 'bool', ?3)",
                        params![id, name, if bv { 1.0 } else { 0.0 }],
                    )?;
                }
                (_, _, Value::Number(n)) => {
                    tx.execute(
                        "INSERT INTO effect_param (effect_id, name, kind, value) VALUES (?1, ?2, 'num', ?3)",
                        params![id, name, n.as_f64().unwrap_or(0.0)],
                    )?;
                }
                (_, n, other) => {
                    return Err(PersistenceError::Corrupt(format!(
                        "parâmetro {n} com tipo inesperado: {other}"
                    )))
                }
            }
        }
    }
    Ok(())
}

pub fn load_slots(conn: &Connection, owner: Owner) -> Result<Vec<EffectSlot>, PersistenceError> {
    let (sql, key): (&str, Option<&str>) = match owner {
        Owner::Track(t) => ("SELECT id, effect_type, bypass FROM effect_slot WHERE track_id = ?1 ORDER BY position", Some(t)),
        Owner::Bus(b) => ("SELECT id, effect_type, bypass FROM effect_slot WHERE bus_id = ?1 ORDER BY position", Some(b)),
        Owner::Master => ("SELECT id, effect_type, bypass FROM effect_slot WHERE on_master = 1 AND ?1 IS NULL ORDER BY position", None),
    };
    let mut stmt = conn.prepare_cached(sql)?;
    let rows: Vec<(String, String, bool)> = stmt
        .query_map(params![key], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    let mut out = Vec::with_capacity(rows.len());
    for (id, ty, bypass) in rows {
        let mut params_obj = Map::new();
        let mut ps =
            conn.prepare_cached("SELECT name, kind, value FROM effect_param WHERE effect_id = ?1")?;
        for row in ps.query_map(params![id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, f64>(2)?,
            ))
        })? {
            let (name, kind, value) = row?;
            let v = if kind == "bool" {
                Value::Bool(value != 0.0)
            } else {
                serde_json::json!(value)
            };
            params_obj.insert(name, v);
        }
        if ty == "eq" {
            let mut bs = conn.prepare_cached(
                "SELECT kind, enabled, freq_hz, gain_db, q FROM eq_band WHERE effect_id = ?1 ORDER BY position",
            )?;
            let bands: Vec<Value> = bs
                .query_map(params![id], |r| {
                    Ok(serde_json::json!({
                        "kind": r.get::<_, String>(0)?,
                        "enabled": r.get::<_, bool>(1)?,
                        "freqHz": r.get::<_, f64>(2)?,
                        "gainDb": r.get::<_, f64>(3)?,
                        "q": r.get::<_, f64>(4)?,
                    }))
                })?
                .collect::<Result<_, _>>()?;
            params_obj.insert("bands".into(), Value::Array(bands));
        }
        let effect: Effect =
            serde_json::from_value(serde_json::json!({ "type": ty, "params": params_obj }))
                .map_err(|e| PersistenceError::Corrupt(format!("efeito {ty}: {e}")))?;
        out.push(EffectSlot {
            id: id
                .parse()
                .map_err(|_| PersistenceError::Corrupt("id de efeito inválido".into()))?,
            bypass,
            effect: effect.clamped(),
        });
    }
    Ok(out)
}
