//! Feitiços de invocador: aplica a dupla mais jogada mantendo cada feitiço na tecla em que já está
//! (o Flash não troca de D para F).

use crate::lcu::Lcu;
use serde_json::json;

pub const FLASH: i64 = 4;

/// `rec` = recomendação; `cur` = (D, F) atuais; `flash_slot` = 1 (D) ou 2 (F) quando não dá para deduzir.
pub fn arrange(rec: (i64, i64), cur: (i64, i64), flash_slot: u8) -> (i64, i64) {
    let r = [rec.0, rec.1];
    let mut out = [0i64; 2];
    if r.contains(&cur.0) {
        out[0] = cur.0;
    }
    if r.contains(&cur.1) && cur.1 != out[0] {
        out[1] = cur.1;
    }
    if out == [0, 0] && r.contains(&FLASH) {
        out[usize::from(flash_slot == 2)] = FLASH;
    }
    let rest: Vec<i64> = r.into_iter().filter(|x| !out.contains(x)).collect();
    let mut rest = rest.into_iter();
    // Recomendação repetida ("4_4"): a tecla que sobra fica como está (antes, `remove(0)`
    // numa lista vazia derrubava o app).
    for (slot, keep) in out.iter_mut().zip([cur.0, cur.1]) {
        if *slot == 0 {
            *slot = rest.next().unwrap_or(keep);
        }
    }
    (out[0], out[1])
}

pub async fn apply(lcu: &Lcu, pair: (i64, i64)) -> Result<String, String> {
    lcu.patch(
        "/lol-champ-select/v1/session/my-selection",
        &json!({"spell1Id": pair.0, "spell2Id": pair.1}),
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(crate::t!("import.spellsApplied"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_keys() {
        // recomendação Flash+Smite, usuário tem Flash no F → Smite no D
        assert_eq!(arrange((4, 11), (14, 4), 1), (11, 4));
        // usuário já tem Flash no D
        assert_eq!(arrange((4, 11), (4, 12), 2), (4, 11));
        // nada em comum: usa a preferência do Flash
        assert_eq!(arrange((4, 11), (3, 7), 2), (11, 4));
        assert_eq!(arrange((4, 11), (3, 7), 1), (4, 11));
        // sem Flash na recomendação
        assert_eq!(arrange((6, 11), (3, 7), 1), (6, 11));
        // já igual
        assert_eq!(arrange((4, 11), (11, 4), 1), (11, 4));
        // recomendação repetida não derruba o app
        assert_eq!(arrange((4, 4), (4, 7), 1), (4, 7));
        assert_eq!(arrange((4, 4), (3, 7), 2), (3, 4));
    }
}
