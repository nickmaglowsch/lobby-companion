//! WebSocket do LCU (WAMP 1.0): eventos em vez de polling.
//! Assina só a fase do gameflow e a sessão da seleção de campeões.

use super::Credentials;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::{Message, client::IntoClientRequest, http::HeaderValue};
use tokio_tungstenite::{Connector, connect_async_tls_with_config};

const TOPIC_PHASE: &str = "OnJsonApiEvent_lol-gameflow_v1_gameflow-phase";
const TOPIC_SESSION: &str = "OnJsonApiEvent_lol-champ-select_v1_session";

#[derive(Debug)]
pub enum LcuEvent {
    Phase(String),
    /// `None` = a sessão acabou (seleção encerrada ou alguém desistiu).
    ChampSelect(Option<Value>),
}

/// Conecta e assina os tópicos. Devolve a tarefa que repassa os eventos até o socket fechar
/// (cliente fechou/reiniciou). Assinar **antes** de ler o estado inicial (GET) evita perder uma
/// troca de fase entre a leitura e a assinatura.
pub async fn subscribe(
    creds: &Credentials,
    tx: mpsc::Sender<LcuEvent>,
) -> Result<impl Future<Output = ()> + Send + 'static, String> {
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
    Ok(async move {
        while let Some(Ok(msg)) = ws.next().await {
            match msg {
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
    })
}

/// `[8, topic, {uri, eventType, data}]` → evento interno.
pub fn parse_event(text: &str) -> Option<LcuEvent> {
    let mut v: Value = serde_json::from_str(text).ok()?;
    let arr = v.as_array_mut()?;
    if arr.first()?.as_i64()? != 8 {
        return None;
    }
    let payload = arr.get_mut(2)?;
    // A sessão chega a cada segundo: tira o `data` do JSON em vez de copiar.
    let data = payload.get_mut("data").map_or(Value::Null, Value::take);
    let delete = payload.get("eventType").and_then(Value::as_str) == Some("Delete");
    match payload.get("uri")?.as_str()? {
        "/lol-gameflow/v1/gameflow-phase" => Some(LcuEvent::Phase(match data {
            Value::String(s) => s,
            _ => "None".into(),
        })),
        "/lol-champ-select/v1/session" => Some(LcuEvent::ChampSelect(
            (!delete && !data.is_null()).then_some(data),
        )),
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
