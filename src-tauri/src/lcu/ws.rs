//! WebSocket do LCU (WAMP 1.0): eventos em vez de polling.
//! Assina só a fase do gameflow e a sessão da seleção de campeões.

use super::Credentials;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::{Message, client::IntoClientRequest, http::HeaderValue};
use tokio_tungstenite::{Connector, connect_async_tls_with_config};

pub const TOPIC_PHASE: &str = "OnJsonApiEvent_lol-gameflow_v1_gameflow-phase";
pub const TOPIC_SESSION: &str = "OnJsonApiEvent_lol-champ-select_v1_session";

#[derive(Debug, Clone)]
pub enum LcuEvent {
    Phase(String),
    /// `None` = a sessão acabou (seleção encerrada ou alguém desistiu).
    ChampSelect(Option<Value>),
}

/// Conecta e repassa eventos até o socket fechar (cliente fechou/reiniciou).
pub async fn run(creds: Credentials, tx: mpsc::Sender<LcuEvent>) -> Result<(), String> {
    let mut req = format!("wss://127.0.0.1:{}/", creds.port)
        .into_client_request()
        .map_err(|e| e.to_string())?;
    let auth =
        HeaderValue::from_str(&creds.basic_auth()).map_err(|_| "header inválido".to_string())?;
    req.headers_mut().insert("Authorization", auth);
    let tls = native_tls::TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .danger_accept_invalid_hostnames(true)
        .build()
        .map_err(|e| e.to_string())?;
    let (mut ws, _) =
        connect_async_tls_with_config(req, None, false, Some(Connector::NativeTls(tls)))
            .await
            .map_err(|e| format!("websocket: {e}"))?;
    for topic in [TOPIC_PHASE, TOPIC_SESSION] {
        ws.send(Message::Text(format!("[5, \"{topic}\"]").into()))
            .await
            .map_err(|e| e.to_string())?;
    }
    while let Some(msg) = ws.next().await {
        match msg.map_err(|e| e.to_string())? {
            Message::Text(t) => {
                if let Some(ev) = parse_event(t.as_str())
                    && tx.send(ev).await.is_err()
                {
                    break;
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
    Ok(())
}

/// `[8, topic, {uri, eventType, data}]` → evento interno.
pub fn parse_event(text: &str) -> Option<LcuEvent> {
    let v: Value = serde_json::from_str(text).ok()?;
    let arr = v.as_array()?;
    if arr.first()?.as_i64()? != 8 {
        return None;
    }
    let payload = arr.get(2)?;
    let uri = payload.get("uri")?.as_str()?;
    let event_type = payload
        .get("eventType")
        .and_then(Value::as_str)
        .unwrap_or("");
    let data = payload.get("data").cloned().unwrap_or(Value::Null);
    match uri {
        "/lol-gameflow/v1/gameflow-phase" => {
            Some(LcuEvent::Phase(data.as_str().unwrap_or("None").to_string()))
        }
        "/lol-champ-select/v1/session" => {
            if event_type == "Delete" || data.is_null() {
                Some(LcuEvent::ChampSelect(None))
            } else {
                Some(LcuEvent::ChampSelect(Some(data)))
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_phase_and_session() {
        let p = r#"[8,"OnJsonApiEvent_lol-gameflow_v1_gameflow-phase",{"data":"ChampSelect","eventType":"Update","uri":"/lol-gameflow/v1/gameflow-phase"}]"#;
        assert!(matches!(parse_event(p), Some(LcuEvent::Phase(s)) if s == "ChampSelect"));
        let d =
            r#"[8,"x",{"data":null,"eventType":"Delete","uri":"/lol-champ-select/v1/session"}]"#;
        assert!(matches!(parse_event(d), Some(LcuEvent::ChampSelect(None))));
        let s = r#"[8,"x",{"data":{"localPlayerCellId":0},"eventType":"Update","uri":"/lol-champ-select/v1/session"}]"#;
        assert!(matches!(
            parse_event(s),
            Some(LcuEvent::ChampSelect(Some(_)))
        ));
        assert!(parse_event("[0, \"welcome\"]").is_none());
        assert!(parse_event("lixo").is_none());
    }
}
