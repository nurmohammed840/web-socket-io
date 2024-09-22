use axum::{extract::ConnectInfo, response::IntoResponse, routing::get, Router};
use axum_socket_io::{SocketIo, SocketIoUpgrade};
use std::{io, net::SocketAddr};
use tokio::net::TcpListener;
use web_socket_io::Procedure;

#[tokio::main]
async fn main() -> io::Result<()> {
    let app = Router::new().route("/ws", get(ws_handler));
    let listener = TcpListener::bind("127.0.0.1:3000").await?;

    println!("listening on {}", listener.local_addr()?);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
}

async fn ws_handler(ws: SocketIoUpgrade, info: ConnectInfo<SocketAddr>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, info.0))
}

async fn handle_socket(mut socket: SocketIo, addr: SocketAddr) {
    println!("A user connected: {addr:#?}");

    while let Ok(ev) = socket.recv().await {
        // socket.join("");
        // socket.leave("");
        // socket.to("room").emit("");

        match ev {
            Procedure::Call(_) => todo!(),
            Procedure::Notify(_) => todo!(),
        }
    }

    println!("user disconnected: {addr:#?}");
}
