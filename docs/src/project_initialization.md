# Project initialization


# Server

Install basic dependency, adding the following line to your `Cargo.toml` file.

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
axum = "0.7"
axum-socket-io = "*"
```

### Basic Server

```rust
use axum::{
    extract::ConnectInfo,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use axum_socket_io::{Procedure, SocketIo, SocketIoUpgrade};
use std::{io, net::SocketAddr};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> io::Result<()> {
    let app = Router::new()
        .route("/", get(|| async { Html(include_str!("../index.html")) }))
        .route("/socket", get(ws_handler));

    let listener = TcpListener::bind("127.0.0.1:3000").await?;
    println!("listening on http://{}", listener.local_addr()?);

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
        // ...
    }

    println!("user disconnected: {addr:#?}");
}
```

### Client Code

```html
<h1>Hello, World!</h1>

<script type="module">
    import { SocketIo } from "https://esm.sh/client-socket-io";

    const socket = new SocketIo("ws://127.0.0.1:3000/socket");
    await socket.connect();

    // ...
</script>
```

