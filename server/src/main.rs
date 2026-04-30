use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{delete, get, post},
    Router,
};
use futures_util::{SinkExt, StreamExt};
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tower_http::services::ServeDir;
use uuid::Uuid;

const SCROLLBACK_LIMIT: usize = 65536; // 64 KB
const BROADCAST_CAP: usize = 256;

#[derive(Clone, Serialize)]
struct SessionInfo {
    id: String,
    name: String,
    created_at: u64,
}

struct Session {
    info: SessionInfo,
    tx: broadcast::Sender<Vec<u8>>,
    scrollback: Arc<Mutex<Vec<u8>>>,
    pty_in_tx: std::sync::mpsc::SyncSender<Vec<u8>>,
    master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
}

type Sessions = Arc<Mutex<HashMap<String, Session>>>;

#[derive(Clone)]
struct AppState {
    sessions: Sessions,
}

#[derive(Deserialize)]
struct CreateSessionQuery {
    name: Option<String>,
}

#[derive(Deserialize)]
struct WsQuery {
    session: String,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum ControlMessage {
    Resize { cols: u16, rows: u16 },
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let static_dir = std::env::var("STATIC_DIR").unwrap_or_else(|_| "static".into());
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".into());

    let state = AppState {
        sessions: Arc::new(Mutex::new(HashMap::new())),
    };

    let app = Router::new()
        .route("/sessions", get(list_sessions))
        .route("/sessions", post(create_session))
        .route("/sessions/:id", delete(delete_session))
        .route("/ws", get(ws_handler))
        .fallback_service(ServeDir::new(&static_dir))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&bind_addr).await.unwrap();
    tracing::info!("Listening on http://{bind_addr}  (static: {static_dir})");
    axum::serve(listener, app).await.unwrap();
}

async fn list_sessions(State(state): State<AppState>) -> Json<Vec<SessionInfo>> {
    let sessions = state.sessions.lock().unwrap();
    let mut infos: Vec<SessionInfo> = sessions.values().map(|s| s.info.clone()).collect();
    infos.sort_by_key(|s| s.created_at);
    Json(infos)
}

async fn create_session(
    State(state): State<AppState>,
    Query(query): Query<CreateSessionQuery>,
) -> Result<Json<SessionInfo>, StatusCode> {
    let id = Uuid::new_v4().to_string();
    let name = query
        .name
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| format!("Session {}", &id[..8]));
    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let session = spawn_session(id.clone(), name, created_at, state.sessions.clone()).map_err(|e| {
        tracing::error!("spawn_session failed: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let info = session.info.clone();
    state.sessions.lock().unwrap().insert(id, session);
    Ok(Json(info))
}

async fn delete_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> StatusCode {
    if state.sessions.lock().unwrap().remove(&id).is_some() {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

fn spawn_session(
    id: String,
    name: String,
    created_at: u64,
    sessions: Sessions,
) -> Result<Session, Box<dyn std::error::Error + Send + Sync>> {
    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into());
    let cmd = CommandBuilder::new(&shell);
    pair.slave.spawn_command(cmd)?;
    // slave はコマンドのスポーン後に解放して master 側の EOF を防ぐ
    drop(pair.slave);

    let master: Arc<Mutex<Box<dyn MasterPty + Send>>> = Arc::new(Mutex::new(pair.master));
    let reader = master.lock().unwrap().try_clone_reader()?;
    let writer = master.lock().unwrap().take_writer()?;

    let (tx, _) = broadcast::channel::<Vec<u8>>(BROADCAST_CAP);
    let scrollback: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));

    // PTY reader → broadcast channel + scrollback ring buffer
    // シェル終了時にレジストリから自動削除する
    let id_for_log = id.clone();
    let id_for_cleanup = id.clone();
    let tx_clone = tx.clone();
    let scrollback_clone = scrollback.clone();
    std::thread::spawn(move || {
        let mut reader = reader;
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let data = buf[..n].to_vec();
                    {
                        let mut sb = scrollback_clone.lock().unwrap();
                        sb.extend_from_slice(&data);
                        if sb.len() > SCROLLBACK_LIMIT {
                            let drain = sb.len() - SCROLLBACK_LIMIT;
                            sb.drain(..drain);
                        }
                    }
                    let _ = tx_clone.send(data);
                }
            }
        }
        sessions.lock().unwrap().remove(&id_for_cleanup);
        tracing::info!("Session {id_for_log} removed after shell exit");
    });

    // PTY writer — stays alive across WebSocket reconnects
    let (pty_in_tx, pty_in_rx) = std::sync::mpsc::sync_channel::<Vec<u8>>(64);
    std::thread::spawn(move || {
        let mut writer = writer;
        while let Ok(data) = pty_in_rx.recv() {
            if writer.write_all(&data).is_err() {
                break;
            }
        }
    });

    Ok(Session {
        info: SessionInfo { id, name, created_at },
        tx,
        scrollback,
        pty_in_tx,
        master,
    })
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<WsQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let (tx, scrollback, pty_in_tx, master) = {
        let sessions = state.sessions.lock().unwrap();
        match sessions.get(&query.session) {
            Some(s) => (
                s.tx.clone(),
                s.scrollback.clone(),
                s.pty_in_tx.clone(),
                s.master.clone(),
            ),
            None => return StatusCode::NOT_FOUND.into_response(),
        }
    };

    ws.on_upgrade(move |socket| handle_socket(socket, tx, scrollback, pty_in_tx, master))
        .into_response()
}

async fn handle_socket(
    socket: WebSocket,
    tx: broadcast::Sender<Vec<u8>>,
    scrollback: Arc<Mutex<Vec<u8>>>,
    pty_in_tx: std::sync::mpsc::SyncSender<Vec<u8>>,
    master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
) {
    let mut rx = tx.subscribe();
    let (mut ws_sink, mut ws_stream) = socket.split();

    // アタッチ時に scrollback を再送してスクリーン状態を復元
    // MutexGuard を await の前に解放するためブロックでスナップショットを取る
    let snapshot = {
        let sb = scrollback.lock().unwrap();
        if sb.is_empty() { None } else { Some(sb.clone()) }
    };
    if let Some(data) = snapshot {
        if ws_sink.send(Message::Binary(data)).await.is_err() {
            return;
        }
    }

    let fwd_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(data) => {
                    if ws_sink.send(Message::Binary(data)).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    while let Some(Ok(msg)) = ws_stream.next().await {
        match msg {
            Message::Binary(data) => {
                let _ = pty_in_tx.send(data);
            }
            Message::Text(text) => {
                if let Ok(ControlMessage::Resize { cols, rows }) = serde_json::from_str(&text) {
                    if let Ok(m) = master.lock() {
                        let _ = m.resize(PtySize {
                            rows,
                            cols,
                            pixel_width: 0,
                            pixel_height: 0,
                        });
                    }
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    fwd_task.abort();
}
