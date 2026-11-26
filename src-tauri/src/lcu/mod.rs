//! Cliente da API local do League Client (LCU).
//!
//! Regras de segurança (ver CLAUDE.md):
//! - o token nunca é impresso, logado nem gravado (`Credentials` não implementa Debug com o token);
//! - TLS sem verificação só para `127.0.0.1` (o certificado do cliente é autoassinado);
//! - escrita (POST/PUT/PATCH/DELETE) só nos caminhos de [`WRITE_ALLOW`]: runas, conjuntos de itens,
//!   feitiços e o ban no clique (`import::ban`, que só manda ação de ban na sua vez).

pub mod ws;

use base64::Engine;
use reqwest::Method;
use serde_json::Value;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug)]
pub enum LcuError {
    NotRunning,
    Forbidden {
        method: String,
        path: String,
    },
    Http {
        status: u16,
        method: String,
        path: String,
        message: String,
    },
    Net(reqwest::Error),
}

/// Mensagem no idioma do app (vai para a janela).
impl std::fmt::Display for LcuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            Self::NotRunning => crate::t!("lcu.notFound"),
            Self::Forbidden { method, path } => {
                crate::t!("lcu.forbidden", method = method, path = path)
            }
            Self::Http {
                status,
                method,
                path,
                message,
            } => crate::t!(
                "lcu.http",
                status = status,
                method = method,
                path = path,
                message = message
            ),
            Self::Net(e) => crate::t!("lcu.network", error = e),
        };
        f.write_str(&msg)
    }
}

impl std::error::Error for LcuError {}

impl From<reqwest::Error> for LcuError {
    fn from(e: reqwest::Error) -> Self {
        Self::Net(e)
    }
}

/// Porta e senha da API local. A senha só sai daqui dentro do header Authorization.
#[derive(Clone)]
pub struct Credentials {
    pub port: u16,
    token: String,
}

impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Credentials {{ port: {}, token: <oculto> }}", self.port)
    }
}

impl Credentials {
    /// Lockfile: `nome:pid:porta:senha:protocolo`.
    pub fn parse_lockfile(text: &str) -> Option<Self> {
        let parts: Vec<&str> = text.trim().split(':').collect();
        if parts.len() < 5 {
            return None;
        }
        let port = parts[2].parse().ok()?;
        let token = parts[3].to_string();
        if token.is_empty() {
            return None;
        }
        Some(Self { port, token })
    }

    pub fn basic_auth(&self) -> String {
        let raw = format!("riot:{}", self.token);
        format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(raw)
        )
    }
}

/// Lugares onde o lockfile costuma estar, do mais confiável ao chute.
fn lockfile_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(p) = std::env::var("LCU_LOCKFILE") {
        out.push(PathBuf::from(p));
    }
    // O Riot Client grava o caminho da instalação aqui.
    let settings = PathBuf::from(
        r"C:\ProgramData\Riot Games\Metadata\league_of_legends.live\league_of_legends.live.product_settings.yaml",
    );
    if let Ok(text) = std::fs::read_to_string(&settings)
        && let Some(dir) = install_path_from_product_settings(&text)
    {
        out.push(PathBuf::from(dir).join("lockfile"));
    }
    for drive in ['C', 'D', 'E', 'F', 'G', 'H'] {
        for sub in [
            r"Riot Games\League of Legends",
            r"Games\Riot Games\League of Legends",
            r"Jogos\Riot Games\League of Legends",
            r"Program Files\Riot Games\League of Legends",
        ] {
            out.push(PathBuf::from(format!(r"{drive}:\{sub}\lockfile")));
        }
    }
    out
}

pub fn install_path_from_product_settings(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let line = line.trim();
        let rest = line.strip_prefix("product_install_full_path:")?;
        let v = rest.trim().trim_matches('"').trim_matches('\'');
        (!v.is_empty()).then(|| v.replace('/', "\\"))
    })
}

/// Procura o cliente aberto. `None` = cliente fechado.
pub fn discover() -> Option<Credentials> {
    lockfile_candidates()
        .into_iter()
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .find_map(|t| Credentials::parse_lockfile(&t))
}

/// Escritas permitidas: (método, prefixo do caminho, caminho exato?).
const WRITE_ALLOW: &[(&str, &str, bool)] = &[
    ("POST", "/lol-perks/v1/pages", true),
    ("DELETE", "/lol-perks/v1/pages/", false),
    ("PUT", "/lol-perks/v1/currentpage", true),
    ("PUT", "/lol-item-sets/v1/item-sets/", false),
    ("PATCH", "/lol-champ-select/v1/session/my-selection", true),
    // Ban no clique (pedido explícito do usuário). `import::ban` só usa com a SUA ação de ban em
    // andamento; picks continuam fora.
    ("PATCH", "/lol-champ-select/v1/session/actions/", false),
];

pub fn write_allowed(method: &str, path: &str) -> bool {
    WRITE_ALLOW.iter().any(|(m, p, exact)| {
        *m == method
            && if *exact {
                path == *p
            } else {
                path.starts_with(p) && path.len() > p.len()
            }
    })
}

#[derive(Clone)]
pub struct Lcu {
    http: reqwest::Client,
    base: String,
    auth: String,
    pub port: u16,
}

impl Lcu {
    pub fn new(creds: &Credentials) -> Result<Self, LcuError> {
        let http = reqwest::Client::builder()
            // Só 127.0.0.1: o certificado do cliente é autoassinado.
            .danger_accept_invalid_certs(true)
            .timeout(Duration::from_secs(10))
            .build()?;
        Ok(Self {
            http,
            base: format!("https://127.0.0.1:{}", creds.port),
            auth: creds.basic_auth(),
            port: creds.port,
        })
    }

    async fn send(
        &self,
        method: Method,
        path: &str,
        body: Option<&Value>,
    ) -> Result<reqwest::Response, LcuError> {
        if method != Method::GET && !write_allowed(method.as_str(), path) {
            return Err(LcuError::Forbidden {
                method: method.to_string(),
                path: path.to_string(),
            });
        }
        let mut req = self
            .http
            .request(method.clone(), format!("{}{}", self.base, path))
            .header(reqwest::header::AUTHORIZATION, &self.auth)
            .header(reqwest::header::ACCEPT, "application/json");
        if let Some(b) = body {
            req = req.json(b);
        }
        let resp = req.send().await?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            let message = serde_json::from_str::<Value>(&text)
                .ok()
                .and_then(|v| v.get("message").and_then(|m| m.as_str()).map(String::from))
                .unwrap_or_else(|| text.chars().take(200).collect());
            return Err(LcuError::Http {
                status: status.as_u16(),
                method: method.to_string(),
                path: path.to_string(),
                message,
            });
        }
        Ok(resp)
    }

    async fn json(
        &self,
        method: Method,
        path: &str,
        body: Option<&Value>,
    ) -> Result<Value, LcuError> {
        let resp = self.send(method, path, body).await?;
        let bytes = resp.bytes().await?;
        if bytes.is_empty() {
            return Ok(Value::Null);
        }
        Ok(serde_json::from_slice(&bytes).unwrap_or(Value::Null))
    }

    pub async fn get(&self, path: &str) -> Result<Value, LcuError> {
        self.json(Method::GET, path, None).await
    }

    pub async fn post(&self, path: &str, body: &Value) -> Result<Value, LcuError> {
        self.json(Method::POST, path, Some(body)).await
    }

    pub async fn put(&self, path: &str, body: &Value) -> Result<Value, LcuError> {
        self.json(Method::PUT, path, Some(body)).await
    }

    pub async fn patch(&self, path: &str, body: &Value) -> Result<Value, LcuError> {
        self.json(Method::PATCH, path, Some(body)).await
    }

    pub async fn delete(&self, path: &str) -> Result<Value, LcuError> {
        self.json(Method::DELETE, path, None).await
    }

    /// Arquivo binário (ícones). Devolve bytes e content-type.
    pub async fn get_bytes(&self, path: &str) -> Result<(Vec<u8>, String), LcuError> {
        let resp = self.send(Method::GET, path, None).await?;
        let ct = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();
        Ok((resp.bytes().await?.to_vec(), ct))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lockfile_parse_and_debug_hides_token() {
        let c = Credentials::parse_lockfile("LeagueClient:1234:54321:s3cr3t:https\n").unwrap();
        assert_eq!(c.port, 54321);
        assert!(!format!("{c:?}").contains("s3cr3t"));
        assert!(c.basic_auth().starts_with("Basic "));
        assert!(Credentials::parse_lockfile("lixo").is_none());
    }

    #[test]
    fn product_settings_path() {
        let y = "product_install_full_path: \"C:/Riot Games/League of Legends\"\nproduct_install_root: \"C:/\"";
        assert_eq!(
            install_path_from_product_settings(y).as_deref(),
            Some(r"C:\Riot Games\League of Legends")
        );
    }

    #[test]
    fn write_allow_list() {
        assert!(write_allowed("POST", "/lol-perks/v1/pages"));
        assert!(write_allowed("DELETE", "/lol-perks/v1/pages/123"));
        assert!(!write_allowed("DELETE", "/lol-perks/v1/pages/"));
        assert!(write_allowed("PUT", "/lol-item-sets/v1/item-sets/42/sets"));
        assert!(write_allowed(
            "PATCH",
            "/lol-champ-select/v1/session/my-selection"
        ));
        // ban no clique (o tipo da ação é conferido em `import::ban`)
        assert!(write_allowed(
            "PATCH",
            "/lol-champ-select/v1/session/actions/1"
        ));
        assert!(!write_allowed(
            "PATCH",
            "/lol-champ-select/v1/session/actions/"
        ));
        assert!(!write_allowed(
            "POST",
            "/lol-champ-select/v1/session/actions/1/complete"
        ));
        // nada de aceitar fila, trocar posição etc.
        assert!(!write_allowed(
            "POST",
            "/lol-matchmaking/v1/ready-check/accept"
        ));
        assert!(!write_allowed(
            "POST",
            "/lol-champ-select/v1/session/my-selection"
        ));
    }
}
