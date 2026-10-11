//! A small Chrome DevTools Protocol client: commands with replies, events for everyone listening.

use anyhow::{anyhow, bail, Result};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, oneshot, Mutex};
use tokio_tungstenite::tungstenite::Message;

type Pending = Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>;

pub struct Cdp {
    pub(super) tx: mpsc::UnboundedSender<Message>,
    pending: Pending,
    events: broadcast::Sender<Value>,
    next: AtomicU64,
}

impl Cdp {
    pub async fn connect(url: &str) -> Result<Cdp> {
        let (socket, _) = tokio_tungstenite::connect_async(url).await?;
        let (mut write, mut read) = socket.split();
        let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
        let pending: Pending = Arc::new(Mutex::new(HashMap::new()));
        let (events, _) = broadcast::channel(1024);

        tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                if write.send(message).await.is_err() {
                    break;
                }
            }
        });

        let pending_reader = pending.clone();
        let events_reader = events.clone();
        tokio::spawn(async move {
            while let Some(Ok(message)) = read.next().await {
                let Message::Text(text) = message else { continue };
                let Ok(value) = serde_json::from_str::<Value>(&text) else { continue };

                if let Some(id) = value["id"].as_u64() {
                    if let Some(sender) = pending_reader.lock().await.remove(&id) {
                        let _ = sender.send(value);
                    }
                } else {
                    let _ = events_reader.send(value);
                }
            }
            // the browser closed: wake everyone waiting
            let _ = events_reader.send(json!({ "method": "Closed" }));
            pending_reader.lock().await.clear();
        });

        Ok(Cdp { tx, pending, events, next: AtomicU64::new(1) })
    }

    pub async fn call(&self, method: &str, params: Value, session: Option<&str>) -> Result<Value> {
        let id = self.next.fetch_add(1, Ordering::SeqCst);
        let (sender, receiver) = oneshot::channel();
        self.pending.lock().await.insert(id, sender);

        let mut message = json!({ "id": id, "method": method, "params": params });
        if let Some(s) = session {
            message["sessionId"] = json!(s);
        }
        self.tx.send(Message::Text(message.to_string())).map_err(|_| anyhow!("The browser was closed"))?;

        let reply = tokio::time::timeout(Duration::from_secs(60), receiver).await.map_err(|_| anyhow!("The browser didn't answer"))?.map_err(|_| anyhow!("The browser was closed"))?;
        if let Some(error) = reply.get("error") {
            bail!("{}", error["message"].as_str().unwrap_or("browser error"));
        }
        Ok(reply["result"].clone())
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Value> {
        self.events.subscribe()
    }
}
