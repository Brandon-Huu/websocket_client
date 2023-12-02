#![allow(unused)]
// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use serde::{Serialize, Deserialize};
use tauri::Manager;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

use futures_util::{future, pin_mut, StreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

#[derive(Serialize,Deserialize,Debug)]
enum CustomMessage{
    PublicKey(Vec<u8>)
}

fn main() {

    tauri::Builder::default()
        .setup(|app| {

            let app_handle = app.handle();

            tauri::async_runtime::spawn(async move {
                let connect_addr = "ws://127.0.0.1:8080".to_string();

                let url = url::Url::parse(&connect_addr).unwrap();

                let (stdin_tx, stdin_rx) = futures_channel::mpsc::unbounded();
                tauri::async_runtime::spawn(read_stdin(stdin_tx));
                let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
                println!("WebSocket handshake has been successfully completed");

                let (write, read) = ws_stream.split();

                let stdin_to_ws = stdin_rx.map(Ok).forward(write);
                let ws_to_stdout = {
                    read.for_each(|message| async {
                        let data = message.unwrap().into_data();
                        match bincode::deserialize::<CustomMessage>(&data[..]) {
                            Ok(response) => println!("{:?}", response),
                            _ => println!("{:?}", data)
                        };
                        //tokio::io::stdout().write_all(&data).await.unwrap();
                    })
                };

                pin_mut!(stdin_to_ws, ws_to_stdout);
                future::select(stdin_to_ws, ws_to_stdout).await;
            });

            Ok(())
        })
        //.invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

async fn read_stdin(tx: futures_channel::mpsc::UnboundedSender<Message>) {
    let mut stdin = tokio::io::stdin();
    loop {
        let mut buf = vec![0; 1024];
        let n = match stdin.read(&mut buf).await {
            Err(_) | Ok(0) => break,
            Ok(n) => n,
        };
        buf.truncate(n);
        tx.unbounded_send(Message::binary(buf)).unwrap();
    }
}
