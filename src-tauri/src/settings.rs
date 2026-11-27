//! Configurações do usuário (JSON na pasta de configuração do app).

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// Filtro de elo do lolalytics: emerald_plus, diamond_plus, master_plus, all…
    pub tier: String,
    /// all, br, euw, kr, na…
    pub region: String,
    /// "" = patch atual; "7"/"30" = últimos dias; "16.23" = patch fixo.
    pub patch: String,
    /// Importa sozinho quando você trava o campeão.
    pub auto_import: bool,
    pub import_runes: bool,
    pub import_items: bool,
    pub import_spells: bool,
    /// "pick" = página mais jogada; "win" = maior win rate.
    pub rune_choice: String,
    /// Amostra mínima para um item entrar no conjunto.
    pub min_games: u64,
    /// Sem espaço para página nova: substituir a página atual (apaga uma página sua).
    pub replace_current_page: bool,
    /// Tecla do Flash quando não dá para deduzir: 1 = D, 2 = F.
    pub flash_slot: u8,
    /// Fecha a janela quando a partida começa (fica só o ícone na bandeja).
    pub close_in_game: bool,
    /// Sugere bans pelos campeões do seu time (um build geral por campeão do time).
    pub suggest_bans: bool,
    /// Sugere pick para a sua posição (counter + sinergia + força na lane).
    pub suggest_picks: bool,
    /// Candidatos da sugestão de pick: "mastery" = seu pool (maestria); "tier" = tier list da lane.
    pub pick_pool: String,
    /// Métrica das sugestões de ban e pick: "wpa" (probabilidade de vitória adicionada) ou
    /// "wr" (pontos de win rate, a conta original). Ver `metric.rs`.
    pub metric: String,
    /// Procura versão nova no GitHub Releases ao abrir e a cada 6 h (só avisa; instalar é no clique).
    pub check_updates: bool,
    /// Layout da janela: "phase" (por fase), "grid" (grade), "tabs" (abas) ou "classic" (lista com rolagem).
    pub layout: String,
    /// Idioma: "" = automático (do cliente do League); senão um locale do cliente ("pt_BR").
    pub language: String,
    /// Último locale lido do cliente (`GET /riotclient/region-locale`): o app abre sem o cliente
    /// e precisa lembrar o idioma.
    pub client_locale: String,
    /// Overlay na partida (`overlay.rs`).
    pub overlay: OverlaySettings,
}

/// Overlay na partida: liga e desliga cada recurso, tecla do placar e posição das janelinhas.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct OverlaySettings {
    pub enabled: bool,
    /// Timer de volta dos inibidores no minimapa.
    pub inhibitors: bool,
    /// Próximo item do build importado quando o ouro dá.
    pub next_item: bool,
    /// Diferença de ouro em itens por rota (com a tecla do placar).
    pub gold_diff: bool,
    /// CS por minuto.
    pub cs_per_min: bool,
    /// Gráfico do ouro em itens dos times (com a tecla do placar).
    pub gold_graph: bool,
    /// Tecla do placar (código virtual do Windows; 9 = Tab).
    pub scoreboard_key: u32,
    /// Posição da janelinha do CS/min (fração da tela, canto superior esquerdo). `None` = padrão.
    pub cs_pos: Option<(f64, f64)>,
    /// Idem para o próximo item.
    pub item_pos: Option<(f64, f64)>,
}

impl Default for OverlaySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            inhibitors: true,
            next_item: true,
            gold_diff: true,
            cs_per_min: true,
            gold_graph: true,
            scoreboard_key: 9,
            cs_pos: None,
            item_pos: None,
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            tier: "emerald_plus".into(),
            region: "all".into(),
            patch: String::new(),
            auto_import: true,
            import_runes: true,
            import_items: true,
            import_spells: true,
            rune_choice: "pick".into(),
            min_games: 200,
            replace_current_page: false,
            flash_slot: 1,
            close_in_game: true,
            suggest_bans: true,
            suggest_picks: true,
            pick_pool: "mastery".into(),
            metric: "wpa".into(),
            check_updates: true,
            layout: "phase".into(),
            language: String::new(),
            client_locale: String::new(),
            overlay: OverlaySettings::default(),
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, serde_json::to_vec_pretty(self).unwrap_or_default())
    }

    /// Tamanho da janela para o layout: (largura, altura, largura mínima, altura mínima).
    /// Os layouts novos cabem numa tela só; o clássico é uma coluna com rolagem.
    pub fn window_size(&self) -> (f64, f64, f64, f64) {
        Self::window_size_for(self.is_classic())
    }

    /// Layout "classic" (coluna com rolagem): o único com outro tamanho de janela.
    pub fn is_classic(&self) -> bool {
        self.layout == "classic"
    }

    pub const fn window_size_for(classic: bool) -> (f64, f64, f64, f64) {
        if classic {
            (480.0, 780.0, 380.0, 480.0)
        } else {
            (720.0, 600.0, 700.0, 580.0)
        }
    }

    /// Idioma em uso: a configuração, senão o do cliente, senão o do sistema, senão inglês.
    pub fn locale(&self, system: Option<&str>) -> &'static str {
        crate::i18n::resolve(&self.language, &self.client_locale, system)
    }

    pub fn patch_opt(&self) -> Option<String> {
        let p = self.patch.trim();
        (!p.is_empty()).then(|| p.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_json_uses_defaults() {
        let s: Settings = serde_json::from_str(r#"{"tier": "diamond_plus"}"#).unwrap();
        assert_eq!(s.tier, "diamond_plus");
        assert_eq!(s.min_games, 200);
        assert!(s.auto_import);
        assert_eq!(s.patch_opt(), None);
        assert!(s.suggest_picks);
        assert_eq!(s.pick_pool, "mastery");
        assert!(s.check_updates);
        assert_eq!(s.layout, "phase");
        assert_eq!(s.metric, "wpa");
        assert_eq!(s.language, "");
        assert_eq!(s.client_locale, "");
        assert_eq!(s.overlay, OverlaySettings::default());
        assert!(s.overlay.enabled && s.overlay.gold_graph);
        assert_eq!(s.overlay.scoreboard_key, 9);
        // Bloco do overlay incompleto: o resto no padrão.
        let s: Settings =
            serde_json::from_str(r#"{"overlay": {"goldDiff": false, "csPos": [0.5, 0.25]}}"#)
                .unwrap();
        assert!(!s.overlay.gold_diff && s.overlay.next_item);
        assert_eq!(s.overlay.cs_pos, Some((0.5, 0.25)));
    }

    #[test]
    fn locale_comes_from_setting_then_client() {
        let mut s = Settings {
            client_locale: "ko_KR".into(),
            ..Default::default()
        };
        assert_eq!(s.locale(Some("pt-BR")), "ko_KR");
        s.language = "ar_AE".into();
        assert_eq!(s.locale(Some("pt-BR")), "ar_AE");
        s.language.clear();
        s.client_locale.clear();
        assert_eq!(s.locale(Some("pt-BR")), "pt_BR");
        assert_eq!(s.locale(None), "en_US");
    }
}
