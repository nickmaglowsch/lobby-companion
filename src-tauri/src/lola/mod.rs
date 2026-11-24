//! Cliente do lolalytics (endpoint não documentado `a1.lolalytics.com/mega/?ep=build-full`).
//!
//! Boa vizinhança: cache em disco (6 h), no máximo 1 requisição por segundo, User-Agent próprio,
//! cópia vencida do cache quando o site recusa (403/429/Cloudflare) e link para a página original na UI.

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

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Query {
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

    /// Página do lolalytics para o usuário abrir (atribuição).
    pub fn page_url(&self) -> String {
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
        format!(
            "{}_{}_{}_{}_{}{vs}.json",
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
                if v.get("header").is_some() {
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
                    stale_warning: Some(format!(
                        "lolalytics indisponível ({e}); usando dados de {} h atrás",
                        age.as_secs() / 3600
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
            .map_err(|e| format!("rede: {e}"))?;
        let status = resp.status();
        if !status.is_success() {
            return Err(format!("HTTP {}", status.as_u16()));
        }
        resp.json::<Value>()
            .await
            .map_err(|e| format!("resposta inválida: {e}"))
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
}
