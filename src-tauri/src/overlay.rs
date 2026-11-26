//! Overlay da partida (opcional, configuração `overlay`, desligada por padrão).
//!
//! Só a API oficial do jogo para apps de terceiros (Live Client Data, `https://127.0.0.1:2999`,
//! só GET) e só dados do próprio jogador: o seu ouro, os seus itens e o seu CS, o tempo de jogo e os
//! eventos de inibidor (que o jogo anuncia a todos). Nada de memória do jogo, nada sobre inimigos
//! que não esteja na tela. O seu Riot ID (para pedir os seus itens e o seu placar) fica só na
//! memória: nunca é gravado nem mostrado.
//!
//! O que aparece: o próximo item da build importada e quanto ouro falta (descontando os componentes
//! que você já tem), o CS por minuto e quanto falta para cada inibidor destruído voltar.

use crate::app::Shared;
use crate::assets::Assets;
use crate::import::itemset::{core_set, is_build_item};
use crate::lola::model::Build;
use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const BASE: &str = "https://127.0.0.1:2999/liveclientdata";
/// Tempo para o inibidor voltar depois de destruído (segundos).
pub const INHIB_RESPAWN: f64 = 300.0;
/// Botas básicas: com elas a build ainda pede as botas da build.
const BASIC_BOOTS: i64 = 1001;
/// Profundidade da receita (item → componente → componente…).
const RECIPE_DEPTH: u8 = 4;

/// Tudo o que a janela do overlay mostra. Vai inteiro no evento "overlay".
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OverlayState {
    pub visible: bool,
    /// Idioma da janela (o mesmo do app).
    pub locale: String,
    pub game_time: f64,
    pub cs: u32,
    pub cs_per_min: f64,
    pub gold: f64,
    pub next: Option<NextItem>,
    pub inhibs: Vec<Inhib>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NextItem {
    pub id: i64,
    pub name: String,
    pub icon: String,
    /// Preço total do item.
    pub price: u32,
    /// Ouro para fechar o item com os componentes que você já tem.
    pub cost: u32,
    /// O que falta além do ouro que você tem agora (0 = dá para comprar).
    pub missing: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Inhib {
    /// "ally" / "enemy" (lado conhecido pela seleção) ou "blue" / "red" (sem a seleção).
    pub side: String,
    /// "top" / "mid" / "bot".
    pub lane: String,
    /// Segundos até voltar.
    pub respawn_in: f64,
}

/// Um instante da partida, só com os seus dados.
#[derive(Debug, Clone, Default)]
pub struct Live {
    pub gold: f64,
    pub game_time: f64,
    pub cs: u32,
    /// Seus itens (um id por unidade).
    pub items: Vec<i64>,
    pub events: Value,
}

/// Ordem dos itens da build em tela: o núcleo (3 itens com botas, na ordem de compra) e depois o
/// mais comprado de cada posição que ainda não entrou. Sem botas no núcleo, as mais compradas vão
/// em segundo.
pub fn build_plan(build: &Build, assets: &Assets, min_games: u64) -> Vec<i64> {
    let is_boots = |id: i64| assets.item_has(id, "Boots");
    let mut plan: Vec<i64> = core_set(build, assets, min_games)
        .map(|(_, ids)| ids)
        .unwrap_or_default();
    for slot in &build.slots {
        let best = slot
            .iter()
            .filter(|i| is_build_item(assets, i.id) && !is_boots(i.id) && !plan.contains(&i.id))
            .max_by_key(|i| i.n);
        if let Some(i) = best {
            plan.push(i.id);
        }
    }
    if !plan.iter().any(|&id| is_boots(id))
        && let Some(b) = build
            .boots
            .iter()
            .filter(|i| assets.items.contains_key(&i.id))
            .max_by_key(|i| i.n)
    {
        plan.insert(plan.len().min(1), b.id);
    }
    plan
}

/// Ouro para fechar `id` com o que está em `inv` (cada componente usado sai da lista).
fn cost_with(assets: &Assets, id: i64, inv: &mut Vec<i64>, depth: u8) -> u32 {
    if let Some(k) = inv.iter().position(|&x| x == id) {
        inv.swap_remove(k);
        return 0;
    }
    let Some(item) = assets.items.get(&id) else {
        return 0;
    };
    if depth == 0 || item.from.is_empty() {
        return item.price;
    }
    let parts: u32 = item
        .from
        .iter()
        .map(|c| assets.items.get(c).map_or(0, |n| n.price))
        .sum();
    let combine = item.price.saturating_sub(parts);
    combine
        + item
            .from
            .iter()
            .map(|&c| cost_with(assets, c, inv, depth - 1))
            .sum::<u32>()
}

/// Primeiro item da build que você ainda não tem e o ouro que falta para ele.
/// Com botas melhoradas (quaisquer), as botas da build contam como compradas.
pub fn next_item(plan: &[i64], owned: &[i64], gold: f64, assets: &Assets) -> Option<NextItem> {
    let has_boots = owned
        .iter()
        .any(|&o| o != BASIC_BOOTS && assets.item_has(o, "Boots"));
    let id = *plan
        .iter()
        .find(|&&id| !owned.contains(&id) && !(has_boots && assets.item_has(id, "Boots")))?;
    let item = assets.items.get(&id)?;
    let mut inv = owned.to_vec();
    let cost = cost_with(assets, id, &mut inv, RECIPE_DEPTH);
    // Ouro do jogo é fracionário; só o inteiro compra.
    let have = gold.max(0.0).floor() as u32;
    Some(NextItem {
        id,
        name: item.name.clone(),
        icon: item.icon.clone(),
        price: item.price,
        cost,
        missing: cost.saturating_sub(have),
    })
}

/// "Barracks_T1_L1" → (1 = azul, "top"); `C` = meio, `R` = baixo.
fn parse_inhib(name: &str) -> Option<(i64, &'static str)> {
    let mut parts = name.split('_');
    if parts.next()? != "Barracks" {
        return None;
    }
    let team = match parts.next()? {
        "T1" => 1,
        "T2" => 2,
        _ => return None,
    };
    let lane = match parts.next()?.chars().next()? {
        'L' => "top",
        'C' => "mid",
        'R' => "bot",
        _ => return None,
    };
    Some((team, lane))
}

/// Inibidores destruídos que ainda não voltaram, pelo `eventdata` (`InhibKilled` /
/// `InhibRespawned`). Lê só o nome do evento, o tempo e o inibidor (nunca quem destruiu).
/// `my_team`: 1 = azul, 2 = vermelho, 0 = não sabemos (mostra a cor).
pub fn inhibs(events: &Value, game_time: f64, my_team: i64) -> Vec<Inhib> {
    let mut down: Vec<(&str, f64)> = Vec::new();
    for e in events
        .get("Events")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let at = e.get("EventTime").and_then(Value::as_f64).unwrap_or(0.0);
        match e.get("EventName").and_then(Value::as_str) {
            Some("InhibKilled") => {
                if let Some(n) = e.get("InhibKilled").and_then(Value::as_str) {
                    down.retain(|(x, _)| *x != n);
                    down.push((n, at + INHIB_RESPAWN));
                }
            }
            Some("InhibRespawned") => {
                if let Some(n) = e.get("InhibRespawned").and_then(Value::as_str) {
                    down.retain(|(x, _)| *x != n);
                }
            }
            _ => {}
        }
    }
    let mut out: Vec<Inhib> = down
        .into_iter()
        .filter(|(_, back)| *back > game_time)
        .filter_map(|(n, back)| {
            let (team, lane) = parse_inhib(n)?;
            let side = match my_team {
                1 | 2 if team == my_team => "ally",
                1 | 2 => "enemy",
                _ if team == 1 => "blue",
                _ => "red",
            };
            Some(Inhib {
                side: side.into(),
                lane: lane.into(),
                respawn_in: back - game_time,
            })
        })
        .collect();
    out.sort_by(|a, b| a.respawn_in.total_cmp(&b.respawn_in));
    out
}

/// Estado do overlay a partir de um instante da partida.
pub fn state(
    live: &Live,
    plan: &[i64],
    assets: Option<&Assets>,
    my_team: i64,
    locale: &str,
) -> OverlayState {
    let minutes = live.game_time / 60.0;
    OverlayState {
        visible: true,
        locale: locale.into(),
        game_time: live.game_time,
        cs: live.cs,
        cs_per_min: if minutes >= 1.0 {
            f64::from(live.cs) / minutes
        } else {
            0.0
        },
        gold: live.gold,
        next: assets.and_then(|a| next_item(plan, &live.items, live.gold, a)),
        inhibs: inhibs(&live.events, live.game_time, my_team),
    }
}

/// Riot ID na query string (`#` e espaços viram `%23`/`%20`).
fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
                char::from(b).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

/// Lista de itens do `playeritems` (um id por unidade).
fn item_ids(v: &Value) -> Vec<i64> {
    v.as_array()
        .into_iter()
        .flatten()
        .flat_map(|i| {
            let id = i.get("itemID").and_then(Value::as_i64).unwrap_or(0);
            let count = i.get("count").and_then(Value::as_u64).unwrap_or(1).min(10);
            std::iter::repeat_n(id, usize::try_from(count).unwrap_or(1))
        })
        .filter(|&id| id > 0)
        .collect()
}

/// Cliente da Live Client Data (só GET, só 127.0.0.1).
pub struct LiveClient {
    http: reqwest::Client,
}

impl LiveClient {
    pub fn new() -> Result<Self, String> {
        let http = reqwest::Client::builder()
            // Só 127.0.0.1: o certificado do jogo é autoassinado.
            .danger_accept_invalid_certs(true)
            .timeout(Duration::from_secs(2))
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self { http })
    }

    async fn get(&self, path: &str) -> Result<Value, String> {
        let resp = self
            .http
            .get(format!("{BASE}{path}"))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(resp.status().to_string());
        }
        resp.json().await.map_err(|e| e.to_string())
    }

    /// Um instante da partida. Falha na tela de carregamento, fora da partida e como espectador.
    pub async fn poll(&self) -> Result<Live, String> {
        let me = self.get("/activeplayer").await?;
        let gold = me
            .get("currentGold")
            .and_then(Value::as_f64)
            .ok_or("activeplayer sem currentGold")?;
        let id = me
            .get("riotId")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or("activeplayer sem riotId")?;
        let q = encode(id);
        let items_path = format!("/playeritems?riotId={q}");
        let scores_path = format!("/playerscores?riotId={q}");
        let (items, scores, stats, events) = tokio::join!(
            self.get(&items_path),
            self.get(&scores_path),
            self.get("/gamestats"),
            self.get("/eventdata"),
        );
        let stats = stats?;
        let cs = scores
            .ok()
            .and_then(|s| s.get("creepScore").and_then(Value::as_u64))
            .map_or(0, |c| u32::try_from(c).unwrap_or(0));
        Ok(Live {
            gold,
            game_time: stats.get("gameTime").and_then(Value::as_f64).unwrap_or(0.0),
            cs,
            items: items.map(|v| item_ids(&v)).unwrap_or_default(),
            events: events.unwrap_or(Value::Null),
        })
    }
}

fn in_game(phase: &str) -> bool {
    matches!(phase, "GameStart" | "InProgress" | "Reconnect")
}

/// Ordem da build do seu campeão (vazia se a janela mostra outro campeão).
async fn current_plan(shared: &Shared, assets: Option<&Assets>, my_champion: i64) -> Vec<i64> {
    let Some(assets) = assets else {
        return Vec::new();
    };
    let configured = shared.settings.lock().await.min_games;
    let snap = shared.snapshot.lock().await;
    match &snap.build {
        Some(b) if my_champion == 0 || my_champion == snap.champion_id => {
            build_plan(b, assets, b.min_games(configured))
        }
        _ => Vec::new(),
    }
}

/// Laço do overlay: com a configuração ligada e a partida em andamento, lê a Live Client Data a
/// cada segundo; mostra a janela quando a API responde e a destrói quando a partida acaba.
pub async fn run(app: AppHandle) {
    let shared = app.state::<Arc<Shared>>().inner().clone();
    let Ok(client) = LiveClient::new() else {
        return;
    };
    let mut shown = false;
    loop {
        let enabled = shared.settings.lock().await.overlay;
        let active = enabled && in_game(&shared.snapshot.lock().await.phase);
        let live = if active {
            client.poll().await.ok()
        } else {
            None
        };
        match live {
            Some(live) => {
                let assets = shared.assets.read().await.clone();
                let (my_champion, my_team) = crate::app::my_champion_team(&shared).await;
                let plan = current_plan(&shared, assets.as_deref(), my_champion).await;
                let locale = shared.snapshot.lock().await.locale.clone();
                let st = state(&live, &plan, assets.as_deref(), my_team, &locale);
                let changed = {
                    let mut cur = shared.overlay.lock().await;
                    let changed = *cur != st;
                    *cur = st.clone();
                    changed
                };
                if !shown {
                    crate::show_overlay(&app);
                    shown = true;
                }
                if changed {
                    let _ = app.emit_to(crate::OVERLAY, "overlay", st);
                }
            }
            None if shown => {
                crate::close_overlay(&app);
                *shared.overlay.lock().await = OverlayState::default();
                shown = false;
            }
            None => {}
        }
        let wait = if active { 1 } else { 2 };
        tokio::time::sleep(Duration::from_secs(wait)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::sample_assets;
    use crate::lola::model::{parse_build_full, tests::fixture};
    use serde_json::json;

    #[test]
    fn plan_is_core_then_most_bought_per_slot() {
        let b = parse_build_full(&fixture()).unwrap();
        let assets = sample_assets();
        // núcleo 3068 → 3047 (botas) → 2502; posição 1: 3084 (mais comprado); posição 2: 3742
        assert_eq!(
            build_plan(&b, &assets, 1000),
            vec![3068, 3047, 2502, 3084, 3742]
        );
        // sem núcleo com amostra: as botas mais compradas entram em segundo
        assert_eq!(build_plan(&b, &assets, 5000), vec![3084, 3009, 3742]);
    }

    #[test]
    fn missing_gold_counts_components_you_have() {
        let assets = sample_assets();
        let plan = [3068, 3047, 2502];
        // nada comprado: Égide inteira (2700)
        let n = next_item(&plan, &[], 1000.4, &assets).unwrap();
        assert_eq!((n.id, n.cost, n.missing), (3068, 2700, 1700));
        // com um Cristal de Rubi (400): falta 2300; com 2500 de ouro, dá para comprar
        let n = next_item(&plan, &[1028], 2500.0, &assets).unwrap();
        assert_eq!((n.cost, n.missing), (2300, 0));
        // Égide pronta: próximo são as botas; as básicas descontam 300
        let n = next_item(&plan, &[3068, 1001], 0.0, &assets).unwrap();
        assert_eq!((n.id, n.cost), (3047, 800));
        // outras botas melhoradas contam como as da build
        let n = next_item(&plan, &[3068, 3009], 0.0, &assets).unwrap();
        assert_eq!(n.id, 2502);
        // build completa: nada
        assert!(next_item(&plan, &[3068, 3047, 2502], 0.0, &assets).is_none());
    }

    #[test]
    fn inhibitor_timers() {
        let ev = json!({"Events": [
            {"EventID": 1, "EventName": "InhibKilled", "EventTime": 1500.0, "KillerName": "x", "InhibKilled": "Barracks_T2_L1"},
            {"EventID": 2, "EventName": "InhibKilled", "EventTime": 1600.0, "InhibKilled": "Barracks_T1_C1"},
            {"EventID": 3, "EventName": "InhibKilled", "EventTime": 1000.0, "InhibKilled": "Barracks_T1_R1"},
            {"EventID": 4, "EventName": "InhibRespawned", "EventTime": 1300.0, "InhibRespawned": "Barracks_T1_R1"},
        ]});
        // você no azul (1): o topo vermelho é do inimigo; o meio azul, seu
        let v = inhibs(&ev, 1700.0, 1);
        assert_eq!(v.len(), 2);
        assert_eq!(
            (v[0].side.as_str(), v[0].lane.as_str(), v[0].respawn_in),
            ("enemy", "top", 100.0)
        );
        assert_eq!(
            (v[1].side.as_str(), v[1].lane.as_str(), v[1].respawn_in),
            ("ally", "mid", 200.0)
        );
        // sem saber o lado: as cores
        let v = inhibs(&ev, 1700.0, 0);
        assert_eq!((v[0].side.as_str(), v[1].side.as_str()), ("red", "blue"));
        // depois de 5 min, voltou
        assert!(inhibs(&ev, 1900.0, 1).iter().all(|i| i.lane != "top"));
        assert!(inhibs(&Value::Null, 10.0, 1).is_empty());
    }

    #[test]
    fn state_and_helpers() {
        let assets = sample_assets();
        let live = Live {
            gold: 500.0,
            game_time: 600.0,
            cs: 75,
            items: item_ids(&json!([{"itemID": 1028, "count": 1}, {"itemID": 2003, "count": 2}])),
            events: Value::Null,
        };
        assert_eq!(live.items, vec![1028, 2003, 2003]);
        let st = state(&live, &[3068], Some(&assets), 1, "pt_BR");
        assert!(st.visible);
        assert!((st.cs_per_min - 7.5).abs() < 1e-9);
        assert_eq!(st.next.unwrap().missing, 1800);
        // antes do primeiro minuto, sem CS/min
        let early = Live {
            game_time: 30.0,
            ..live
        };
        assert_eq!(state(&early, &[], None, 0, "en_US").cs_per_min, 0.0);
        assert_eq!(encode("Nome Legal#BR1"), "Nome%20Legal%23BR1");
        assert_eq!(parse_inhib("Barracks_T2_R1"), Some((2, "bot")));
        assert_eq!(parse_inhib("Turret_T1_L_03_A"), None);
    }
}
