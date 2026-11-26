//! Idioma do app: mensagens que vão para a janela, o menu da bandeja e o que vai para o cliente
//! (nome da página de runas, títulos dos blocos do conjunto de itens).
//!
//! Escolha (`resolve`): a configuração "Idioma", senão o último locale lido do cliente do League
//! (`GET /riotclient/region-locale`, guardado nas configurações), senão o idioma do sistema,
//! senão inglês. Os textos ficam em `src-tauri/locales/<arquivo>.json` (os da janela, em
//! `src/lib/i18n/locales/`); variantes iguais dividem o arquivo (`file_for`).

use std::collections::HashMap;
use std::fmt::Display;
use std::sync::{OnceLock, RwLock};

/// Locales do cliente do League (ddragon `languages.json`, conferido em 2026-11-26).
pub const LOCALES: &[&str] = &[
    "en_US", "en_GB", "en_AU", "en_PH", "en_SG", "es_ES", "es_MX", "es_AR", "pt_BR", "fr_FR",
    "de_DE", "it_IT", "pl_PL", "ro_RO", "el_GR", "hu_HU", "cs_CZ", "ru_RU", "tr_TR", "ja_JP",
    "ko_KR", "zh_CN", "zh_MY", "zh_TW", "th_TH", "vi_VN", "id_ID", "ar_AE",
];

pub const FALLBACK: &str = "en_US";

macro_rules! files {
    ($($name:literal),+ $(,)?) => {
        &[$(($name, include_str!(concat!("../locales/", $name, ".json")))),+]
    };
}

/// Arquivo de tradução → conteúdo (embutido no binário).
const FILES: &[(&str, &str)] = files!(
    "en", "es_ES", "es_MX", "pt_BR", "fr", "de", "it", "pl", "ro", "el", "hu", "cs", "ru", "tr",
    "ja", "ko", "zh_CN", "zh_TW", "th", "vi", "id", "ar",
);

/// Arquivo de tradução de um locale do cliente (o mesmo mapeamento de `src/lib/i18n`).
pub fn file_for(locale: &str) -> &'static str {
    match locale {
        l if l.starts_with("en_") => "en",
        "es_MX" | "es_AR" => "es_MX",
        "es_ES" => "es_ES",
        "pt_BR" => "pt_BR",
        "zh_CN" | "zh_MY" => "zh_CN",
        "zh_TW" => "zh_TW",
        "fr_FR" => "fr",
        "de_DE" => "de",
        "it_IT" => "it",
        "pl_PL" => "pl",
        "ro_RO" => "ro",
        "el_GR" => "el",
        "hu_HU" => "hu",
        "cs_CZ" => "cs",
        "ru_RU" => "ru",
        "tr_TR" => "tr",
        "ja_JP" => "ja",
        "ko_KR" => "ko",
        "th_TH" => "th",
        "vi_VN" => "vi",
        "id_ID" => "id",
        "ar_AE" => "ar",
        _ => "en",
    }
}

type Dict = HashMap<String, String>;

fn dicts() -> &'static HashMap<&'static str, Dict> {
    static D: OnceLock<HashMap<&'static str, Dict>> = OnceLock::new();
    D.get_or_init(|| {
        FILES
            .iter()
            .map(|(name, src)| (*name, serde_json::from_str(src).unwrap_or_default()))
            .collect()
    })
}

/// Um locale do cliente exatamente como ele manda ("pt_BR"; aceita "pt-BR").
pub fn supported(locale: &str) -> Option<&'static str> {
    let norm = locale.trim().replace('-', "_");
    LOCALES
        .iter()
        .find(|l| l.eq_ignore_ascii_case(&norm))
        .copied()
}

/// Tag de idioma qualquer (sistema: "pt-BR", "en", "zh-Hant-TW", "es-419") → locale do cliente
/// mais próximo; `None` se o idioma não existe no cliente.
pub fn from_tag(tag: &str) -> Option<&'static str> {
    if let Some(l) = supported(tag) {
        return Some(l);
    }
    let tag = tag.trim().replace('-', "_");
    let tag = tag.split(['.', '@']).next().unwrap_or_default(); // "pt_BR.UTF-8"
    let mut parts = tag.split('_');
    let lang = parts.next()?.to_ascii_lowercase();
    let rest: Vec<String> = parts.map(str::to_ascii_uppercase).collect();
    let script = rest.iter().find(|p| p.len() == 4).cloned();
    let region = rest.iter().find(|p| p.len() != 4).cloned();
    if let Some(r) = &region
        && let Some(l) = supported(&format!("{lang}_{r}"))
    {
        return Some(l);
    }
    let r = region.as_deref().unwrap_or_default();
    Some(match lang.as_str() {
        "en" => "en_US",
        // Espanhol fora da Espanha = América Latina (o cliente tem es_MX e es_AR).
        "es" if r.is_empty() || r == "ES" => "es_ES",
        "es" => "es_MX",
        "pt" => "pt_BR",
        "zh" if script.as_deref() == Some("HANT") || matches!(r, "TW" | "HK" | "MO") => "zh_TW",
        "zh" => "zh_CN",
        "ar" => "ar_AE",
        other => LOCALES
            .iter()
            .find(|l| l.split('_').next() == Some(other))?,
    })
}

/// Idioma em uso: configuração → cliente (último visto) → sistema → inglês.
pub fn resolve(setting: &str, client: &str, system: Option<&str>) -> &'static str {
    supported(setting)
        .or_else(|| supported(client).or_else(|| from_tag(client)))
        .or_else(|| system.and_then(from_tag))
        .unwrap_or(FALLBACK)
}

/// Idioma do sistema operacional ("pt-BR"), se der para ler.
pub fn system_locale() -> Option<String> {
    sys_locale::get_locale()
}

static CURRENT: RwLock<&'static str> = RwLock::new(FALLBACK);

/// Troca o idioma das mensagens; devolve `true` se mudou.
pub fn set_current(locale: &'static str) -> bool {
    let mut cur = CURRENT.write().unwrap_or_else(|e| e.into_inner());
    let changed = *cur != locale;
    *cur = locale;
    changed
}

/// Idioma em uso pelo app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lang {
    pub locale: &'static str,
}

impl Lang {
    pub fn current() -> Self {
        Self {
            locale: *CURRENT.read().unwrap_or_else(|e| e.into_inner()),
        }
    }

    pub fn of(locale: &str) -> Self {
        Self {
            locale: supported(locale).unwrap_or(FALLBACK),
        }
    }

    fn lookup(&self, key: &str) -> Option<&'static str> {
        let d = dicts();
        d.get(file_for(self.locale))
            .and_then(|m| m.get(key))
            .or_else(|| d.get("en").and_then(|m| m.get(key)))
            .map(String::as_str)
    }

    /// Texto da chave (sem parâmetros). Chave que falta em todos os arquivos: a própria chave.
    pub fn t(&self, key: &str) -> String {
        self.lookup(key).unwrap_or(key).to_string()
    }

    /// Texto com `{nome}` trocado por cada parâmetro. Com `count`, usa a forma de plural do idioma
    /// (`chave_one`, `chave_few`, `chave_other`…) quando existe.
    pub fn tf(&self, key: &str, args: &[(&str, &dyn Display)]) -> String {
        let count = args
            .iter()
            .find(|(k, _)| *k == "count")
            .and_then(|(_, v)| v.to_string().parse::<u64>().ok());
        let template = count
            .and_then(|n| {
                self.lookup(&format!("{key}_{}", plural(self.locale, n)))
                    .or_else(|| self.lookup(&format!("{key}_other")))
            })
            .or_else(|| self.lookup(key))
            .unwrap_or(key);
        let mut out = template.to_string();
        for (name, value) in args {
            out = out.replace(&format!("{{{name}}}"), &value.to_string());
        }
        out
    }

    /// `53.456` → "53,5%" (pt), "53.5%" (en), "%53,5" (tr): uma casa, separador e formato do idioma.
    pub fn pct(&self, x: f64) -> String {
        let n = format!("{x:.1}").replace('.', &self.t("fmt.decimal"));
        self.t("fmt.percent").replace("{n}", &n)
    }
}

/// Categoria de plural (CLDR, só inteiros) para escolher `chave_<categoria>`.
pub fn plural(locale: &str, n: u64) -> &'static str {
    let lang = locale.split('_').next().unwrap_or_default();
    let (m10, m100) = (n % 10, n % 100);
    match lang {
        "ja" | "ko" | "zh" | "th" | "vi" | "id" => "other",
        "fr" | "pt" if n <= 1 => "one",
        "ru" if m10 == 1 && m100 != 11 => "one",
        "ru" | "pl" if (2..=4).contains(&m10) && !(12..=14).contains(&m100) => "few",
        "ru" => "many",
        "pl" if n == 1 => "one",
        "pl" => "many",
        "cs" if n == 1 => "one",
        "cs" if (2..=4).contains(&n) => "few",
        "ro" if n == 1 => "one",
        "ro" if n == 0 || (2..=19).contains(&m100) => "few",
        "ar" => match n {
            0 => "zero",
            1 => "one",
            2 => "two",
            _ if (3..=10).contains(&m100) => "few",
            _ if (11..=99).contains(&m100) => "many",
            _ => "other",
        },
        _ if n == 1 => "one",
        _ => "other",
    }
}

/// `t!("chave")` / `t!("chave", nome = valor, …)` no idioma em uso.
#[macro_export]
macro_rules! t {
    ($key:expr) => {
        $crate::i18n::Lang::current().t($key)
    };
    ($key:expr, $($name:ident = $val:expr),+ $(,)?) => {
        $crate::i18n::Lang::current().tf($key, &[$((stringify!($name), &$val as &dyn ::std::fmt::Display)),+])
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn every_client_locale_has_a_file() {
        assert_eq!(LOCALES.len(), 28);
        for l in LOCALES {
            let f = file_for(l);
            assert!(FILES.iter().any(|(n, _)| *n == f), "{l} → {f}");
        }
        assert_eq!(file_for("en_GB"), "en");
        assert_eq!(file_for("es_AR"), "es_MX");
        assert_eq!(file_for("zh_MY"), "zh_CN");
        assert_eq!(file_for("pt_BR"), "pt_BR");
        assert_eq!(file_for("xx_YY"), "en");
    }

    #[test]
    fn system_tags_map_to_client_locales() {
        assert_eq!(from_tag("pt-BR"), Some("pt_BR"));
        assert_eq!(from_tag("pt-PT"), Some("pt_BR"));
        assert_eq!(from_tag("en"), Some("en_US"));
        assert_eq!(from_tag("en-GB"), Some("en_GB"));
        assert_eq!(from_tag("en-IN"), Some("en_US"));
        assert_eq!(from_tag("es-419"), Some("es_MX"));
        assert_eq!(from_tag("es-AR"), Some("es_AR"));
        assert_eq!(from_tag("es"), Some("es_ES"));
        assert_eq!(from_tag("zh-Hant-TW"), Some("zh_TW"));
        assert_eq!(from_tag("zh-HK"), Some("zh_TW"));
        assert_eq!(from_tag("zh-Hans"), Some("zh_CN"));
        assert_eq!(from_tag("ar-SA"), Some("ar_AE"));
        assert_eq!(from_tag("de"), Some("de_DE"));
        assert_eq!(from_tag("ja-JP"), Some("ja_JP"));
        assert_eq!(from_tag("pt_BR.UTF-8"), Some("pt_BR"));
        assert_eq!(from_tag("nl-NL"), None);
        assert_eq!(from_tag(""), None);
    }

    #[test]
    fn resolve_prefers_setting_then_client_then_system() {
        assert_eq!(resolve("ja_JP", "pt_BR", Some("de-DE")), "ja_JP");
        assert_eq!(resolve("", "pt_BR", Some("de-DE")), "pt_BR");
        assert_eq!(resolve("", "", Some("de-DE")), "de_DE");
        assert_eq!(resolve("", "", Some("nl-NL")), "en_US");
        assert_eq!(resolve("", "", None), "en_US");
        // Configuração inválida (arquivo editado à mão) cai no automático.
        assert_eq!(resolve("klingon", "ko_KR", None), "ko_KR");
        // Locale novo do cliente que ainda não conhecemos: o idioma mais próximo.
        assert_eq!(resolve("", "es_US", None), "es_MX");
    }

    #[test]
    fn plurals_follow_the_language() {
        assert_eq!(plural("en_US", 1), "one");
        assert_eq!(plural("en_US", 0), "other");
        assert_eq!(plural("pt_BR", 0), "one");
        assert_eq!(plural("ru_RU", 21), "one");
        assert_eq!(plural("ru_RU", 3), "few");
        assert_eq!(plural("ru_RU", 11), "many");
        assert_eq!(plural("pl_PL", 22), "few");
        assert_eq!(plural("pl_PL", 5), "many");
        assert_eq!(plural("ja_JP", 1), "other");
        assert_eq!(plural("ar_AE", 2), "two");
        assert_eq!(plural("ar_AE", 11), "many");
    }

    #[test]
    fn texts_with_params_and_fallback() {
        let pt = Lang::of("pt_BR");
        assert_eq!(pt.t("lane.jungle"), "Selva");
        assert_eq!(Lang::of("en_GB").t("lane.jungle"), "Jungle");
        assert_eq!(pt.pct(53.456), "53,5%");
        assert_eq!(Lang::of("en_US").pct(53.456), "53.5%");
        assert_eq!(Lang::of("tr_TR").pct(53.456), "%53,5");
        assert_eq!(pt.t("chave.que.nao.existe"), "chave.que.nao.existe");
        assert_eq!(
            pt.tf("items.saved", &[("count", &1)]),
            "conjunto gravado (1 bloco)"
        );
        assert_eq!(
            pt.tf("items.saved", &[("count", &9)]),
            "conjunto gravado (9 blocos)"
        );
    }

    fn placeholders(s: &str) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        let mut rest = s;
        while let Some(i) = rest.find('{') {
            let Some(j) = rest[i..].find('}') else { break };
            out.insert(rest[i + 1..i + j].to_string());
            rest = &rest[i + j + 1..];
        }
        out
    }

    /// Chave sem o sufixo de plural (`games_one` → `games`).
    fn base(key: &str) -> &str {
        for suf in ["_zero", "_one", "_two", "_few", "_many", "_other"] {
            if let Some(b) = key.strip_suffix(suf) {
                return b;
            }
        }
        key
    }

    /// Cada arquivo tem as chaves do inglês (plural pode ter outras formas) e os mesmos `{params}`.
    fn check_same_keys(files: &[(String, Dict)]) {
        let en = &files.iter().find(|(n, _)| n == "en").expect("en.json").1;
        let en_bases: BTreeSet<&str> = en.keys().map(|k| base(k)).collect();
        for (name, d) in files {
            let bases: BTreeSet<&str> = d.keys().map(|k| base(k)).collect();
            let missing: Vec<_> = en_bases.difference(&bases).collect();
            let extra: Vec<_> = bases.difference(&en_bases).collect();
            assert!(missing.is_empty(), "{name}: faltam {missing:?}");
            assert!(extra.is_empty(), "{name}: sobram {extra:?}");
            for (k, v) in d {
                let reference = en
                    .get(k)
                    .or_else(|| en.get(&format!("{}_other", base(k))))
                    .or_else(|| en.get(base(k)));
                if let Some(r) = reference {
                    let (mut want, mut got) = (placeholders(r), placeholders(v));
                    // Forma de plural sem o número ("um bloco") pode omitir {count}.
                    if k != base(k) {
                        want.remove("count");
                        got.remove("count");
                    }
                    assert_eq!(want, got, "{name}: {k} = {v:?}");
                }
                assert!(!v.trim().is_empty(), "{name}: {k} vazio");
            }
        }
    }

    #[test]
    fn backend_translations_are_complete() {
        let files: Vec<(String, Dict)> = FILES
            .iter()
            .map(|(n, s)| {
                let d: Dict = serde_json::from_str(s).unwrap_or_else(|e| panic!("{n}.json: {e}"));
                (n.to_string(), d)
            })
            .collect();
        check_same_keys(&files);
    }

    /// Os arquivos da janela (`src/lib/i18n/locales`) seguem as mesmas regras e existem para
    /// cada arquivo do backend.
    #[test]
    fn window_translations_are_complete() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/lib/i18n/locales");
        let files: Vec<(String, Dict)> = FILES
            .iter()
            .map(|(n, _)| {
                let path = dir.join(format!("{n}.json"));
                let src = std::fs::read_to_string(&path)
                    .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
                let d: Dict =
                    serde_json::from_str(&src).unwrap_or_else(|e| panic!("{n}.json: {e}"));
                (n.to_string(), d)
            })
            .collect();
        check_same_keys(&files);
    }
}
