pub mod room;
mod socket;

use axum::{
    extract::ConnectInfo,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use axum_socket_io::SocketIoUpgrade;
use std::{io, net::SocketAddr, sync::LazyLock};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> io::Result<()> {
    let app = Router::new()
        .route("/", get(|| async { Html(include_str!("../index.html")) }))
        .route("/socket", get(ws_handler));

    LazyLock::force(&room::TASK);
    println!("listening on http://127.0.0.1:3000");

    axum::serve(
        TcpListener::bind("127.0.0.1:3000").await?,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
}

async fn ws_handler(ws: SocketIoUpgrade, info: ConnectInfo<SocketAddr>) -> impl IntoResponse {
    ws.on_upgrade(16, move |socket| socket::handle_socket(socket, info.0))
}
