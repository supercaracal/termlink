use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use serde::Deserialize;
use tokio::sync::mpsc;
use tower_http::services::ServeDir;

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

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .fallback_service(ServeDir::new(&static_dir));

    let listener = tokio::net::TcpListener::bind(&bind_addr).await.unwrap();
    tracing::info!("Listening on http://{bind_addr}  (static: {static_dir})");
    axum::serve(listener, app).await.unwrap();
}

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(socket: WebSocket) {
    let pty_system = native_pty_system();
    let pair = match pty_system.openpty(PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    }) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("openpty failed: {e}");
            return;
        }
    };

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into());
    let cmd = CommandBuilder::new(&shell);
    let _child = match pair.slave.spawn_command(cmd) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("spawn_command failed: {e}");
            return;
        }
    };
    // slave はコマンドのスポーン後に解放して master 側の EOF を防ぐ
    drop(pair.slave);

    let master: Arc<Mutex<Box<dyn MasterPty + Send>>> = Arc::new(Mutex::new(pair.master));

    let reader = master.lock().unwrap().try_clone_reader().unwrap();
    let writer = master.lock().unwrap().take_writer().unwrap();

    // PTY 出力 → チャンネル → WebSocket
    let (pty_out_tx, mut pty_out_rx) = mpsc::channel::<Vec<u8>>(64);
    std::thread::spawn(move || {
        let mut reader = reader;
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if pty_out_tx.blocking_send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
            }
        }
    });

    // WebSocket 入力 → チャンネル → PTY
    let (pty_in_tx, pty_in_rx) = std::sync::mpsc::sync_channel::<Vec<u8>>(64);
    std::thread::spawn(move || {
        let mut writer = writer;
        while let Ok(data) = pty_in_rx.recv() {
            if writer.write_all(&data).is_err() {
                break;
            }
        }
    });

    let (mut ws_sink, mut ws_stream) = socket.split();

    // PTY 出力チャンネル → WebSocket 送信タスク
    let fwd_task = tokio::spawn(async move {
        while let Some(data) = pty_out_rx.recv().await {
            if ws_sink
                .send(Message::Binary(data))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    // WebSocket 受信 → PTY 入力 / リサイズ
    while let Some(Ok(msg)) = ws_stream.next().await {
        match msg {
            Message::Binary(data) => {
                let _ = pty_in_tx.send(data);
            }
            Message::Text(text) => {
                if let Ok(ControlMessage::Resize { cols, rows }) =
                    serde_json::from_str(&text)
                {
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
