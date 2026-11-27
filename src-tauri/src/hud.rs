//! Onde o jogo desenha o minimapa e o placar do Tab, para o overlay acertar sem ajuste manual.
//!
//! Medidas tiradas dos arquivos de layout do próprio jogo (CommunityDragon, patch de 2026-11:
//! `clientstates/gameplay/ux/lol/lolminimap/uibase` e `uiflipped`, `clientstates/gameplay/ux/
//! scoreboard/uibase`). Cada elemento tem posição e tamanho numa tela de referência 1600×1200 e
//! uma âncora (0..1 em cada eixo: canto ou centro). O jogo estica tudo pela mesma escala
//! `k = min(largura/1600, altura/1200)` (16:9, 21:9 e 4:3 → altura/1200; só telas mais estreitas
//! que 4:3 usam a largura) e mantém cada elemento preso à sua âncora: em 21:9 o minimapa continua
//! no canto e o placar no centro. Conferido num print 1920×1080 (erro de 1 a 3 px).
//!
//! Minimapa e placar ignoram a escala da interface (`IgnoreGlobalScale`). O minimapa tem a própria
//! escala (`MinimapScale` no `game.cfg`, 1.0 = 100 %, pode passar de 1) e cresce a partir do canto
//! em que está preso.

use serde::Serialize;

pub const REF_W: f64 = 1600.0;
pub const REF_H: f64 = 1200.0;

/// Retângulo em pixels da tela do jogo (origem no canto superior esquerdo da área de desenho).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub fn center(&self) -> (f64, f64) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }
}

/// Elemento do layout do jogo, em coordenadas de 1600×1200.
#[derive(Debug, Clone, Copy)]
struct Elem {
    pos: (f64, f64),
    size: (f64, f64),
    anchor: (f64, f64),
}

/// Mapa do minimapa (`MinimapContent`), preso no canto inferior direito.
const MINIMAP_RIGHT: Elem = Elem {
    pos: (1305.0, 903.0),
    size: (283.0, 283.0),
    anchor: (1.0, 1.0),
};
/// Com "minimapa à esquerda" (`uiflipped`): preso no canto inferior esquerdo.
const MINIMAP_LEFT: Elem = Elem {
    pos: (13.0, 903.0),
    size: (285.0, 285.0),
    anchor: (0.0, 1.0),
};
/// Fundo do placar do Tab (`SB_BG`), centralizado.
const SCOREBOARD: Elem = Elem {
    pos: (156.0, 262.0),
    size: (1286.0, 517.0),
    anchor: (0.5, 0.5),
};
/// Primeira linha de jogador (`SB_PlayerSlotHeightRef`); as outras descem de 84 em 84.
const SCOREBOARD_ROW0_Y: f64 = 346.0;
const SCOREBOARD_ROW_H: f64 = 84.0;
/// Centro do divisor entre os dois times (`SB_DividerOverlay`: x 777, largura 45).
const SCOREBOARD_DIVIDER_X: f64 = 777.0 + 45.0 / 2.0;

/// Configuração do HUD que muda a posição do que o overlay usa.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HudConfig {
    /// 1.0 = 100 %.
    pub minimap_scale: f64,
    pub minimap_left: bool,
}

impl Default for HudConfig {
    fn default() -> Self {
        Self {
            minimap_scale: 1.0,
            minimap_left: false,
        }
    }
}

/// Escala do HUD para uma área de desenho `w`×`h`.
pub fn ui_scale(w: f64, h: f64) -> f64 {
    (w / REF_W).min(h / REF_H)
}

/// Posiciona um elemento: a âncora vai para o mesmo ponto relativo da tela, e a distância até ela
/// é multiplicada pela escala (`extra` = escala própria do elemento, como a do minimapa).
fn place(e: Elem, w: f64, h: f64, extra: f64) -> Rect {
    let k = ui_scale(w, h) * extra;
    let (ax, ay) = (e.anchor.0 * REF_W, e.anchor.1 * REF_H);
    let (sx, sy) = (e.anchor.0 * w, e.anchor.1 * h);
    Rect {
        x: sx + (e.pos.0 - ax) * k,
        y: sy + (e.pos.1 - ay) * k,
        w: e.size.0 * k,
        h: e.size.1 * k,
    }
}

/// Área do mapa no minimapa (sem a moldura).
pub fn minimap(w: f64, h: f64, cfg: &HudConfig) -> Rect {
    let e = if cfg.minimap_left {
        MINIMAP_LEFT
    } else {
        MINIMAP_RIGHT
    };
    let scale = if cfg.minimap_scale.is_finite() && cfg.minimap_scale > 0.0 {
        cfg.minimap_scale
    } else {
        1.0
    };
    place(e, w, h, scale)
}

/// Fundo do placar do Tab.
pub fn scoreboard(w: f64, h: f64) -> Rect {
    place(SCOREBOARD, w, h, 1.0)
}

/// Ponto entre as duas colunas do placar, na altura da linha `row` (0 = de cima).
pub fn scoreboard_row_center(row: usize, w: f64, h: f64) -> (f64, f64) {
    let y = SCOREBOARD_ROW0_Y + SCOREBOARD_ROW_H * row as f64 + SCOREBOARD_ROW_H / 2.0;
    let p = place(
        Elem {
            pos: (SCOREBOARD_DIVIDER_X, y),
            size: (0.0, 0.0),
            anchor: SCOREBOARD.anchor,
        },
        w,
        h,
        1.0,
    );
    (p.x, p.y)
}

/// O que o overlay lê do `Config/game.cfg` do jogo (formato INI).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameCfg {
    /// Resolução salva (a área de desenho real vem da janela do jogo; isto é só o plano B).
    pub width: Option<u32>,
    pub height: Option<u32>,
    /// 0 = tela cheia, 1 = janela, 2 = sem bordas (valores comuns; só para o diagnóstico).
    pub window_mode: Option<u32>,
    pub hud: HudConfig,
}

/// Lê o `game.cfg`. Chaves sem diferença de maiúsculas; o que faltar fica no padrão.
pub fn parse_game_cfg(text: &str) -> GameCfg {
    let mut out = GameCfg::default();
    let mut section = String::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if let Some(s) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = s.trim().to_ascii_lowercase();
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let (k, v) = (k.trim().to_ascii_lowercase(), v.trim());
        match (section.as_str(), k.as_str()) {
            ("general", "width") => out.width = v.parse().ok(),
            ("general", "height") => out.height = v.parse().ok(),
            ("general", "windowmode") => out.window_mode = v.parse().ok(),
            ("hud", "minimapscale") => {
                if let Ok(s) = v.parse::<f64>() {
                    out.hud.minimap_scale = s;
                }
            }
            // "Mostrar minimapa à esquerda" nas opções do jogo.
            ("hud", "flipminimap") => out.hud.minimap_left = truthy(v),
            _ => {}
        }
    }
    out
}

fn truthy(v: &str) -> bool {
    matches!(v.to_ascii_lowercase().as_str(), "1" | "true")
}

/// Modo de vídeo "Sem bordas" (`WindowMode` do `[General]`).
pub const BORDERLESS: u32 = 2;

/// Troca `WindowMode=0` (tela cheia, onde janela nenhuma aparece por cima) por "Sem bordas" no
/// `[General]`. `None` = nada a mudar (já em janela ou sem bordas, ou sem a chave). Mantém o
/// resto do arquivo como está, inclusive as quebras de linha.
pub fn set_borderless(text: &str) -> Option<String> {
    let mut section = String::new();
    let mut changed = false;
    let mut out = String::with_capacity(text.len() + 1);
    for raw in text.split_inclusive('\n') {
        let body = raw.trim_end_matches(['\r', '\n']);
        let ending = &raw[body.len()..];
        let line = body.trim();
        if let Some(s) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = s.trim().to_ascii_lowercase();
        } else if section == "general"
            && let Some((k, v)) = line.split_once('=')
            && k.trim().eq_ignore_ascii_case("windowmode")
            && v.trim() == "0"
        {
            out.push_str(&format!("{}={BORDERLESS}{ending}", k.trim()));
            changed = true;
            continue;
        }
        out.push_str(raw);
    }
    changed.then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fullscreen_becomes_borderless_and_nothing_else_changes() {
        let cfg = "[General]\r\nWidth=2560\r\nWindowMode=0\r\n\r\n[HUD]\r\nWindowMode=0\r\n";
        let out = set_borderless(cfg).unwrap();
        assert_eq!(
            out,
            "[General]\r\nWidth=2560\r\nWindowMode=2\r\n\r\n[HUD]\r\nWindowMode=0\r\n"
        );
        assert_eq!(parse_game_cfg(&out).window_mode, Some(BORDERLESS));
        // Já em janela ou sem bordas, sem a chave ou chave noutra seção: não mexe.
        assert!(set_borderless("[General]\nWindowMode=1\n").is_none());
        assert!(set_borderless("[General]\nWindowMode=2").is_none());
        assert!(set_borderless("[General]\nWidth=1920\n").is_none());
        assert!(set_borderless("[Replay]\nWindowMode=0\n").is_none());
        // Última linha sem quebra.
        assert_eq!(
            set_borderless("[general]\nwindowmode = 0").as_deref(),
            Some("[general]\nwindowmode=2")
        );
    }

    fn near(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn minimap_1080p_matches_screenshot() {
        // Print real 1920×1080, minimapa em 100 %: mapa de ~1653 a ~1911 em x e ~810 a ~1070 em y.
        let r = minimap(1920.0, 1080.0, &HudConfig::default());
        assert!(near(r.x, 1654.5, 0.1) && near(r.y, 812.7, 0.1), "{r:?}");
        assert!(near(r.w, 254.7, 0.1) && near(r.x + r.w, 1909.2, 0.1));
        assert!(near(r.y + r.h, 1067.4, 0.1));
    }

    #[test]
    fn minimap_stays_in_the_corner_on_ultrawide_and_4_3() {
        // 21:9: mesma escala do 1080p, colado no canto direito.
        let uw = minimap(2560.0, 1080.0, &HudConfig::default());
        let hd = minimap(1920.0, 1080.0, &HudConfig::default());
        assert!(near(uw.w, hd.w, 1e-9) && near(uw.y, hd.y, 1e-9));
        assert!(near(2560.0 - (uw.x + uw.w), 1920.0 - (hd.x + hd.w), 1e-9));
        // 4:3 na resolução de referência: escala 1, posição igual à dos arquivos do jogo.
        let r = minimap(1600.0, 1200.0, &HudConfig::default());
        assert_eq!(
            r,
            Rect {
                x: 1305.0,
                y: 903.0,
                w: 283.0,
                h: 283.0
            }
        );
        // 5:4 é mais estreito que 4:3: a largura manda.
        assert!(near(ui_scale(1280.0, 1024.0), 0.8, 1e-9));
    }

    #[test]
    fn minimap_left_and_scale_grow_from_the_corner() {
        let left = HudConfig {
            minimap_scale: 1.0,
            minimap_left: true,
        };
        let r = minimap(1920.0, 1080.0, &left);
        assert!(near(r.x, 13.0 * 0.9, 1e-9) && near(r.w, 285.0 * 0.9, 1e-9));
        // Escala 1,5: o canto preso não sai do lugar.
        let base = minimap(1920.0, 1080.0, &HudConfig::default());
        let big = minimap(
            1920.0,
            1080.0,
            &HudConfig {
                minimap_scale: 1.5,
                minimap_left: false,
            },
        );
        assert!(near(big.w, base.w * 1.5, 1e-9));
        assert!(near(
            1920.0 - (big.x + big.w),
            (1920.0 - (base.x + base.w)) * 1.5,
            1e-9
        ));
        // Valor inválido no arquivo vira 100 %.
        let bad = HudConfig {
            minimap_scale: 0.0,
            minimap_left: false,
        };
        assert_eq!(minimap(1920.0, 1080.0, &bad), base);
    }

    #[test]
    fn scoreboard_1080p_matches_screenshot() {
        // Print real: caixa de ~383 a 1537 em x e ~235 a 698 em y; linhas a cada 76 px.
        let sb = scoreboard(1920.0, 1080.0);
        assert!(near(sb.x, 380.4, 0.1) && near(sb.y, 235.8, 0.1), "{sb:?}");
        assert!(near(sb.x + sb.w, 1537.8, 0.1) && near(sb.y + sb.h, 701.1, 0.1));
        let (x0, y0) = scoreboard_row_center(0, 1920.0, 1080.0);
        let (_, y1) = scoreboard_row_center(1, 1920.0, 1080.0);
        assert!(near(x0, 959.55, 0.01) && near(y0, 349.2, 0.1), "{x0} {y0}");
        assert!(near(y1 - y0, 75.6, 1e-9));
    }

    #[test]
    fn scoreboard_is_centered_on_ultrawide() {
        let (x, _) = scoreboard_row_center(0, 2560.0, 1080.0);
        assert!(near(x, 1280.0 - 0.45, 0.01));
        let sb = scoreboard(2560.0, 1080.0);
        assert!(near(sb.center().0, 1280.0 - 0.9, 0.01));
    }

    #[test]
    fn game_cfg_parsing() {
        let cfg = parse_game_cfg(
            "[General]\nWidth=2560\nHeight=1080\nWindowMode=2\n\n[HUD]\nMinimapScale=0.8500\nflipminimap=1\nGlobalScale=0.5\n",
        );
        assert_eq!(cfg.width, Some(2560));
        assert_eq!(cfg.height, Some(1080));
        assert_eq!(cfg.window_mode, Some(2));
        assert!(near(cfg.hud.minimap_scale, 0.85, 1e-9));
        assert!(cfg.hud.minimap_left);
        // Sem seção HUD: padrão (100 %, direita).
        assert_eq!(
            parse_game_cfg("[General]\nWidth=1920\n").hud,
            HudConfig::default()
        );
        // Chave igual em outra seção não conta.
        assert!(!parse_game_cfg("[Replay]\nFlipMiniMap=1\n").hud.minimap_left);
    }
}
