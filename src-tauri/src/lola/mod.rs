//! Cliente do lolalytics (endpoints não documentados `a1.lolalytics.com/mega/?ep=…`):
//! `build-full` (build do campeão, ou do confronto no `a3`), `list` (tier list de uma lane),
//! `counter` (WR do campeão contra cada oponente de uma lane) e `build-team` (WR do campeão
//! com cada aliado de cada lane). Formatos e significados em `extra.rs` e no CLAUDE.md.
//!
//! Boa vizinhança: cache em disco (6 h), no máximo 1 requisição por segundo, User-Agent próprio,
//! cópia vencida do cache quando o site recusa (403/429/Cloudflare) e link para a página original na UI.

pub mod extra;
pub mod model;

use serde_json::Value;
use std::fmt::{self, Write as _};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::Mutex;

const API: &str = "https://a1.lolalytics.com/mega/";
/// Confronto (`/lol/<c>/vs/<vs>/build/`): outro servidor, mesmo formato, sem região (sempre global).
const API_VS: &str = "https://a3.lolalytics.com/mega/";
const TTL: Duration = Duration::from_secs(6 * 3600);
const MIN_GAP: Duration = Duration::from_millis(1000);
pub const USER_AGENT: &str = concat!(
    "lobby-companion/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/nickmaglowsch/lobby-companion)"
);

/// Qual endpoint do `mega/` a consulta usa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Ep {
    /// Build geral (ou de confronto, com `vs`).
    #[default]
    BuildFull,
    /// Tier list da `lane` (todos os campeões; `slug` vazio).
    List,
    /// Oponentes do `slug` (na `lane`) que jogam na `vs_lane`.
    Counter,
    /// Aliados do `slug` (na `lane`) em cada lane.
    BuildTeam,
}

impl Ep {
    fn name(self) -> &'static str {
        match self {
            Ep::BuildFull => "build-full",
            Ep::List => "list",
            Ep::Counter => "counter",
            Ep::BuildTeam => "build-team",
        }
    }

    /// A resposta tem os dados (e pode ir para o cache)? Erros vêm como `{"status": 404x}`.
    fn valid(self, v: &Value) -> bool {
        let key = match self {
            Ep::BuildFull => "header",
            Ep::List => "cid",
            Ep::Counter => "counters",
            Ep::BuildTeam => "team",
        };
        v.get(key).is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Query {
    pub ep: Ep,
    pub slug: String,
    /// `None` = lane mais jogada do campeão (o lolalytics escolhe).
    pub lane: Option<String>,
    pub tier: String,
    pub region: String,
    /// `None` = patch atual; também aceita "7"/"30" (últimos dias) ou "16.23".
    pub patch: Option<String>,
    /// Campeão do outro lado (slug): build do confronto. A região é ignorada (sempre global).
    pub vs: Option<String>,
    /// Lane do campeão do outro lado (`None` = a mais comum dele contra você).
    pub vs_lane: Option<String>,
}

impl Query {
    pub fn api_url(&self) -> String {
        let mut url = String::with_capacity(160);
        let (slug, tier, region) = (Enc(&self.slug), Enc(&self.tier), Enc(&self.region));
        match (self.ep, &self.vs) {
            (Ep::BuildFull, None) => {
                let _ = write!(
                    url,
                    "{API}?ep=build-full&v=1&c={slug}&tier={tier}&queue=ranked&region={region}"
                );
                param(&mut url, "lane", self.lane.as_deref());
            }
            (Ep::BuildFull, Some(vs)) => {
                let _ = write!(
                    url,
                    "{API_VS}?ep=build-full&v=1&c={slug}&vs={}&tier={tier}&queue=ranked",
                    Enc(vs)
                );
                param(&mut url, "lane", self.lane.as_deref());
                param(&mut url, "vslane", self.vs_lane.as_deref());
            }
            // `list`, `counter` e `build-team`: sempre no `a1`, com região.
            (ep, _) => {
                let _ = write!(url, "{API}?ep={}&v=1", ep.name());
                if ep != Ep::List {
                    let _ = write!(url, "&c={slug}");
                }
                param(&mut url, "lane", self.lane.as_deref());
                if ep == Ep::Counter {
                    param(&mut url, "vslane", self.vs_lane.as_deref());
                }
                let _ = write!(url, "&tier={tier}&queue=ranked&region={region}");
            }
        }
        param(&mut url, "patch", self.patch.as_deref());
        url
    }

    /// Página do lolalytics para o usuário abrir (atribuição).
    pub fn page_url(&self) -> String {
        let mut url = String::with_capacity(120);
        let (slug, tier) = (Enc(&self.slug), Enc(&self.tier));
        // A sinergia (`build-team`) aparece na página do build do campeão.
        let build = matches!(self.ep, Ep::BuildFull | Ep::BuildTeam);
        let _ = match (self.ep, &self.vs) {
            (Ep::List, _) => write!(url, "https://lolalytics.com/lol/tierlist/?tier={tier}"),
            (Ep::Counter, _) => write!(
                url,
                "https://lolalytics.com/lol/{slug}/counters/?tier={tier}"
            ),
            (_, None) => write!(url, "https://lolalytics.com/lol/{slug}/build/?tier={tier}"),
            (_, Some(vs)) => write!(
                url,
                "https://lolalytics.com/lol/{slug}/vs/{}/build/?tier={tier}",
                Enc(vs)
            ),
        };
        let vs_build = build && self.vs.is_some();
        param(&mut url, "lane", self.lane.as_deref());
        if vs_build || self.ep == Ep::Counter {
            param(&mut url, "vslane", self.vs_lane.as_deref());
        }
        // O confronto não tem região (sempre global).
        if !vs_build && self.region != "all" {
            param(&mut url, "region", Some(&self.region));
        }
        param(&mut url, "patch", self.patch.as_deref());
        url
    }

    fn cache_key(&self) -> String {
        fn or_default(o: Option<&str>) -> &str {
            o.unwrap_or("default")
        }
        let mut key = String::with_capacity(96);
        // O build geral mantém a chave antiga (o cache de quem atualiza continua valendo).
        let _ = match self.ep {
            Ep::BuildFull => Ok(()),
            Ep::Counter => write!(key, "counter_{}_", or_default(self.vs_lane.as_deref())),
            e => write!(key, "{}_", e.name()),
        };
        let _ = write!(
            key,
            "{}_{}_{}_{}_{}",
            self.slug,
            or_default(self.lane.as_deref()),
            self.tier,
            if self.vs.is_some() {
                "all"
            } else {
                &self.region
            },
            self.patch.as_deref().unwrap_or("atual")
        );
        if let Some(v) = &self.vs {
            let _ = write!(key, "_vs_{v}_{}", or_default(self.vs_lane.as_deref()));
        }
        key.push_str(".json");
        key.chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' || c == '.' {
                    c
                } else {
                    '-'
                }
            })
            .collect()
    }
}

/// Só deixa passar o que aparece nos parâmetros (slugs, tiers, regiões, patches), sem alocar.
struct Enc<'a>(&'a str);

impl fmt::Display for Enc<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
            .try_for_each(|c| f.write_char(c))
    }
}

/// `&nome=valor` quando há valor.
fn param(url: &mut String, name: &str, value: Option<&str>) {
    if let Some(v) = value {
        let _ = write!(url, "&{name}={}", Enc(v));
    }
}

pub struct Fetched {
    pub data: Value,
    /// `Some(aviso)` quando veio do cache vencido porque o site falhou.
    pub stale_warning: Option<String>,
}

pub struct Lolalytics {
    http: reqwest::Client,
    cache_dir: PathBuf,
    /// Início da última requisição. Quem busca no site fica com o lock do começo ao fim (espera
    /// do intervalo, requisição e gravação no cache): as requisições saem uma por vez, a ≥1 s uma
    /// da outra, e quem esperava pela mesma chave acha o cache novo em vez de repetir a busca.
    last: Mutex<Option<Instant>>,
}

impl Lolalytics {
    pub fn new(cache_dir: PathBuf) -> Self {
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(15))
            .build()
            .expect("cliente HTTP");
        Self {
            http,
            cache_dir,
            last: Mutex::new(None),
        }
    }

    /// Idade do arquivo do cache (sem ler o conteúdo).
    fn cache_age(path: &Path) -> Option<Duration> {
        let modified = std::fs::metadata(path).ok()?.modified().ok()?;
        Some(
            SystemTime::now()
                .duration_since(modified)
                .unwrap_or_default(),
        )
    }

    fn read_cache(path: &Path) -> Option<Value> {
        serde_json::from_slice(&std::fs::read(path).ok()?).ok()
    }

    /// O cache, se tem menos de 6 h e dá para ler.
    fn fresh(path: &Path) -> Option<Fetched> {
        if Self::cache_age(path)? >= TTL {
            return None;
        }
        Some(Fetched {
            data: Self::read_cache(path)?,
            stale_warning: None,
        })
    }

    /// Qualquer endpoint: cache de 6 h, depois o site; se o site falhar, o cache vencido.
    pub async fn get(&self, q: &Query) -> Result<Fetched, String> {
        let path = self.cache_dir.join(q.cache_key());
        if let Some(hit) = Self::fresh(&path) {
            return Ok(hit);
        }
        let mut last = self.last.lock().await;
        // Outra tarefa pode ter buscado a mesma chave enquanto esta esperava a vez.
        if let Some(hit) = Self::fresh(&path) {
            return Ok(hit);
        }
        if let Some(wait) = last.and_then(|t| MIN_GAP.checked_sub(t.elapsed())) {
            tokio::time::sleep(wait).await;
        }
        *last = Some(Instant::now());
        let result = self.fetch(q, &path).await;
        drop(last);
        result
            .map(|data| Fetched {
                data,
                stale_warning: None,
            })
            .or_else(
                |e| match (Self::cache_age(&path), Self::read_cache(&path)) {
                    (Some(age), Some(data)) => Ok(Fetched {
                        data,
                        stale_warning: Some(crate::t!(
                            "lola.stale",
                            error = e,
                            hours = age.as_secs() / 3600
                        )),
                    }),
                    _ => Err(e),
                },
            )
    }

    /// Busca no site e grava no cache a resposta como veio (sem serializar de novo), num arquivo
    /// ao lado que depois troca de nome: quem lê o cache nunca vê um arquivo pela metade.
    async fn fetch(&self, q: &Query, path: &Path) -> Result<Value, String> {
        let resp = self
            .http
            .get(q.api_url())
            .send()
            .await
            .map_err(|e| crate::t!("lola.network", error = e))?;
        let status = resp.status();
        if !status.is_success() {
            return Err(format!("HTTP {}", status.as_u16()));
        }
        let invalid = |e: &dyn fmt::Display| crate::t!("lola.invalid", error = e);
        let body = resp.bytes().await.map_err(|e| invalid(&e))?;
        let data: Value = serde_json::from_slice(&body).map_err(|e| invalid(&e))?;
        // Erros "de dados" ({"status": 404x}) não vão para o cache.
        if q.ep.valid(&data) {
            let _ = crate::assets::write_atomic(path, &body);
        }
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls() {
        let q = Query {
            slug: "chogath".into(),
            lane: Some("jungle".into()),
            tier: "emerald_plus".into(),
            region: "all".into(),
            ..Default::default()
        };
        assert_eq!(
            q.api_url(),
            "https://a1.lolalytics.com/mega/?ep=build-full&v=1&c=chogath&tier=emerald_plus&queue=ranked&region=all&lane=jungle"
        );
        assert_eq!(
            q.page_url(),
            "https://lolalytics.com/lol/chogath/build/?tier=emerald_plus&lane=jungle"
        );
        assert_eq!(q.cache_key(), "chogath_jungle_emerald_plus_all_atual.json");
        let vs = Query {
            vs: Some("viego".into()),
            vs_lane: Some("jungle".into()),
            region: "br".into(),
            ..q.clone()
        };
        assert_eq!(
            vs.api_url(),
            "https://a3.lolalytics.com/mega/?ep=build-full&v=1&c=chogath&vs=viego&tier=emerald_plus&queue=ranked&lane=jungle&vslane=jungle"
        );
        assert_eq!(
            vs.page_url(),
            "https://lolalytics.com/lol/chogath/vs/viego/build/?tier=emerald_plus&lane=jungle&vslane=jungle"
        );
        assert_eq!(
            vs.cache_key(),
            "chogath_jungle_emerald_plus_all_atual_vs_viego_jungle.json"
        );
        let evil = Query {
            slug: "a&b=../c".into(),
            ..q
        };
        assert!(!evil.api_url().contains("&b="));
        assert!(!evil.cache_key().contains('/'));
    }

    #[test]
    fn extra_endpoint_urls() {
        let base = Query {
            tier: "emerald_plus".into(),
            region: "all".into(),
            ..Default::default()
        };
        let list = Query {
            ep: Ep::List,
            lane: Some("jungle".into()),
            ..base.clone()
        };
        assert_eq!(
            list.api_url(),
            "https://a1.lolalytics.com/mega/?ep=list&v=1&lane=jungle&tier=emerald_plus&queue=ranked&region=all"
        );
        assert_eq!(
            list.page_url(),
            "https://lolalytics.com/lol/tierlist/?tier=emerald_plus&lane=jungle"
        );
        assert_eq!(list.cache_key(), "list__jungle_emerald_plus_all_atual.json");
        let counter = Query {
            ep: Ep::Counter,
            slug: "viego".into(),
            lane: Some("jungle".into()),
            vs_lane: Some("middle".into()),
            ..base.clone()
        };
        assert_eq!(
            counter.api_url(),
            "https://a1.lolalytics.com/mega/?ep=counter&v=1&c=viego&lane=jungle&vslane=middle&tier=emerald_plus&queue=ranked&region=all"
        );
        assert_eq!(
            counter.page_url(),
            "https://lolalytics.com/lol/viego/counters/?tier=emerald_plus&lane=jungle&vslane=middle"
        );
        assert_eq!(
            counter.cache_key(),
            "counter_middle_viego_jungle_emerald_plus_all_atual.json"
        );
        let team = Query {
            ep: Ep::BuildTeam,
            slug: "lux".into(),
            lane: Some("middle".into()),
            patch: Some("7".into()),
            ..base
        };
        assert_eq!(
            team.api_url(),
            "https://a1.lolalytics.com/mega/?ep=build-team&v=1&c=lux&lane=middle&tier=emerald_plus&queue=ranked&region=all&patch=7"
        );
        assert_eq!(
            team.page_url(),
            "https://lolalytics.com/lol/lux/build/?tier=emerald_plus&lane=middle&patch=7"
        );
        // chave diferente do build geral do mesmo campeão
        assert_eq!(
            team.cache_key(),
            "build-team_lux_middle_emerald_plus_all_7.json"
        );
        assert!(Ep::Counter.valid(&serde_json::json!({"counters": []})));
        assert!(!Ep::List.valid(&serde_json::json!({"status": 4044})));
    }

    #[test]
    fn fresh_cache_skips_the_site() {
        let dir = std::env::temp_dir().join(format!("lc-lola-{}", std::process::id()));
        let q = Query {
            ep: Ep::List,
            lane: Some("jungle".into()),
            tier: "emerald_plus".into(),
            region: "all".into(),
            ..Default::default()
        };
        std::fs::create_dir_all(&dir).unwrap();
        // A resposta vai para o cache como veio do site.
        std::fs::write(dir.join(q.cache_key()), br#"{"cid": {}}"#).unwrap();
        let lola = Lolalytics::new(dir.clone());
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let f = rt.block_on(lola.get(&q)).unwrap();
        assert!(f.stale_warning.is_none());
        assert!(f.data.get("cid").is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
