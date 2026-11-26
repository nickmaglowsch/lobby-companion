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
        match self.layout.as_str() {
            "classic" => (480.0, 780.0, 380.0, 480.0),
            _ => (720.0, 600.0, 700.0, 580.0),
        }
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
    }
}
