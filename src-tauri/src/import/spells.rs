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
    let mut rest: Vec<i64> = r.iter().copied().filter(|x| !out.contains(x)).collect();
    if out == [0, 0] && rest.contains(&FLASH) {
        let flash_idx = if flash_slot == 2 { 1 } else { 0 };
        out[flash_idx] = FLASH;
        rest.retain(|x| *x != FLASH);
    }
    for slot in out.iter_mut() {
        if *slot == 0 {
            *slot = rest.remove(0);
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
    Ok("feitiços aplicados".into())
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
    }
}
