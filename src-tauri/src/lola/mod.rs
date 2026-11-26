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
use std::path::PathBuf;
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
        if self.ep != Ep::BuildFull {
            return self.extra_api_url();
        }
        let mut url = match &self.vs {
            None => format!(
                "{API}?ep=build-full&v=1&c={}&tier={}&queue=ranked&region={}",
                enc(&self.slug),
                enc(&self.tier),
                enc(&self.region)
            ),
            Some(vs) => format!(
                "{API_VS}?ep=build-full&v=1&c={}&vs={}&tier={}&queue=ranked",
                enc(&self.slug),
                enc(vs),
                enc(&self.tier)
            ),
        };
        if let Some(l) = &self.lane {
            url.push_str(&format!("&lane={}", enc(l)));
        }
        if self.vs.is_some()
            && let Some(l) = &self.vs_lane
        {
            url.push_str(&format!("&vslane={}", enc(l)));
        }
        if let Some(p) = &self.patch {
            url.push_str(&format!("&patch={}", enc(p)));
        }
        url
    }

    /// `list`, `counter` e `build-team`: sempre no `a1`, com região.
    fn extra_api_url(&self) -> String {
        let mut url = format!("{API}?ep={}&v=1", self.ep.name());
        if self.ep != Ep::List {
            url.push_str(&format!("&c={}", enc(&self.slug)));
        }
        if let Some(l) = &self.lane {
            url.push_str(&format!("&lane={}", enc(l)));
        }
        if self.ep == Ep::Counter
            && let Some(l) = &self.vs_lane
        {
            url.push_str(&format!("&vslane={}", enc(l)));
        }
        url.push_str(&format!(
            "&tier={}&queue=ranked&region={}",
            enc(&self.tier),
            enc(&self.region)
        ));
        if let Some(p) = &self.patch {
            url.push_str(&format!("&patch={}", enc(p)));
        }
        url
    }

    /// Página do lolalytics para o usuário abrir (atribuição).
    pub fn page_url(&self) -> String {
        match self.ep {
            Ep::BuildFull => {}
            Ep::List | Ep::Counter => {
                let mut url = if self.ep == Ep::List {
                    format!(
                        "https://lolalytics.com/lol/tierlist/?tier={}",
                        enc(&self.tier)
                    )
                } else {
                    format!(
                        "https://lolalytics.com/lol/{}/counters/?tier={}",
                        enc(&self.slug),
                        enc(&self.tier)
                    )
                };
                if let Some(l) = &self.lane {
                    url.push_str(&format!("&lane={}", enc(l)));
                }
                if self.ep == Ep::Counter
                    && let Some(l) = &self.vs_lane
                {
                    url.push_str(&format!("&vslane={}", enc(l)));
                }
                if self.region != "all" {
                    url.push_str(&format!("&region={}", enc(&self.region)));
                }
                if let Some(p) = &self.patch {
                    url.push_str(&format!("&patch={}", enc(p)));
                }
                return url;
            }
            // A sinergia aparece na página do build do campeão.
            Ep::BuildTeam => {
                return Query {
                    ep: Ep::BuildFull,
                    ..self.clone()
                }
                .page_url();
            }
        }
        let mut url = match &self.vs {
            None => format!(
                "https://lolalytics.com/lol/{}/build/?tier={}",
                enc(&self.slug),
                enc(&self.tier)
            ),
            Some(vs) => format!(
                "https://lolalytics.com/lol/{}/vs/{}/build/?tier={}",
                enc(&self.slug),
                enc(vs),
                enc(&self.tier)
            ),
        };
        if let Some(l) = &self.lane {
            url.push_str(&format!("&lane={}", enc(l)));
        }
        if self.vs.is_some() {
            if let Some(l) = &self.vs_lane {
                url.push_str(&format!("&vslane={}", enc(l)));
            }
        } else if self.region != "all" {
            url.push_str(&format!("&region={}", enc(&self.region)));
        }
        if let Some(p) = &self.patch {
            url.push_str(&format!("&patch={}", enc(p)));
        }
        url
    }

    fn cache_key(&self) -> String {
        let vs = match &self.vs {
            None => String::new(),
            Some(v) => format!("_vs_{v}_{}", self.vs_lane.as_deref().unwrap_or("default")),
        };
        // O build geral mantém a chave antiga (o cache de quem atualiza continua valendo).
        let ep = match self.ep {
            Ep::BuildFull => String::new(),
            Ep::Counter => format!(
                "{}_{}_",
                self.ep.name(),
                self.vs_lane.as_deref().unwrap_or("default")
            ),
            e => format!("{}_", e.name()),
        };
        format!(
            "{ep}{}_{}_{}_{}_{}{vs}.json",
            self.slug,
            self.lane.as_deref().unwrap_or("default"),
            self.tier,
            if self.vs.is_some() {
                "all"
            } else {
                &self.region
            },
            self.patch.as_deref().unwrap_or("atual")
        )
        .chars()
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

/// Só deixa passar o que aparece nos parâmetros (slugs, tiers, regiões, patches).
fn enc(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
        .collect()
}

pub struct Fetched {
    pub data: Value,
    /// `Some(aviso)` quando veio do cache vencido porque o site falhou.
    pub stale_warning: Option<String>,
}

pub struct Lolalytics {
    http: reqwest::Client,
    cache_dir: PathBuf,
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

    fn read_cache(&self, q: &Query) -> Option<(Value, Duration)> {
        let p = self.cache_dir.join(q.cache_key());
        let meta = std::fs::metadata(&p).ok()?;
        let age = SystemTime::now()
            .duration_since(meta.modified().ok()?)
            .unwrap_or_default();
        let v = serde_json::from_slice(&std::fs::read(&p).ok()?).ok()?;
        Some((v, age))
    }

    fn write_cache(&self, q: &Query, v: &Value) {
        let _ = std::fs::create_dir_all(&self.cache_dir);
        let p = self.cache_dir.join(q.cache_key());
        let tmp = p.with_extension("tmp");
        if std::fs::write(&tmp, serde_json::to_vec(v).unwrap_or_default()).is_ok() {
            let _ = std::fs::rename(&tmp, &p);
        }
    }

    pub async fn build_full(&self, q: &Query) -> Result<Fetched, String> {
        self.get(q).await
    }

    /// Qualquer endpoint: cache de 6 h, depois o site; se o site falhar, o cache vencido.
    pub async fn get(&self, q: &Query) -> Result<Fetched, String> {
        let cached = self.read_cache(q);
        if let Some((v, age)) = &cached
            && *age < TTL
        {
            return Ok(Fetched {
                data: v.clone(),
                stale_warning: None,
            });
        }
        match self.fetch(q).await {
            Ok(v) => {
                // Erros "de dados" ({"status": 404x}) não vão para o cache.
                if q.ep.valid(&v) {
                    self.write_cache(q, &v);
                }
                Ok(Fetched {
                    data: v,
                    stale_warning: None,
                })
            }
            Err(e) => match cached {
                Some((v, age)) => Ok(Fetched {
                    data: v,
                    stale_warning: Some(crate::t!(
                        "lola.stale",
                        error = e,
                        hours = age.as_secs() / 3600
                    )),
                }),
                None => Err(e),
            },
        }
    }

    async fn fetch(&self, q: &Query) -> Result<Value, String> {
        {
            let mut last = self.last.lock().await;
            if let Some(t) = *last {
                let since = t.elapsed();
                if since < MIN_GAP {
                    tokio::time::sleep(MIN_GAP - since).await;
                }
            }
            *last = Some(Instant::now());
        }
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
        resp.json::<Value>()
            .await
            .map_err(|e| crate::t!("lola.invalid", error = e))
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
}
