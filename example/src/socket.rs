use crate::room::*;
use axum_socket_io::{Procedure, SocketIo};
use std::{net::SocketAddr, time::Duration};
use tokio::time::sleep;

pub async fn handle_socket(mut socket: SocketIo, addr: SocketAddr) {
    let id = addr.port();
    let notifier = socket.notifier();
    Action::Join { id, notifier }.dispatch().await;
    println!("A user connected: {addr:#?}");

    while let Ok(ev) = socket.recv().await {
        match ev {
            Procedure::Notify(req) => match req.method() {
                "ping" => socket.notify("pong", req.data()).await.unwrap(),
                "broadcast" => Action::Broadcast(req.data().into()).dispatch().await,
                _ => {}
            },
            Procedure::Call(req, res, c) => match req.method() {
                "myip" => res.send(addr.to_string()).await.unwrap(),
                "uppercase" => {
                    let msg = std::str::from_utf8(req.data()).unwrap();
                    res.send(msg.to_uppercase()).await.unwrap()
                }
                "long_runing_task" => {
                    c.spawn_and_abort_on_reset(async {
                        sleep(Duration::from_secs(3)).await;
                        res.send("done!").await.unwrap();
                    });
                }
                _ => {}
            },
        }
    }

    println!("user disconnected: {addr:#?}");
    Action::Leave { id }.dispatch().await;
}
