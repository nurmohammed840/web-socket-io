# Tutorial

This tutorial demonstrates how to set up a basic server using the Rust web
framework [axum](https://github.com/tokio-rs/axum), along with
[axum-socket-io](https://github.com/nurmohammed840/web-socket-io) for real-time
communication.

### Setup

1. Initialize a new Rust project

```bash
cargo new <project-name>
cd <project-name>
```

2. Add dependencies to `Cargo.toml`:

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
axum = "0.7"
axum-socket-io = "0.1"
```

### Server Implementation

In the `src/socket.rs` file, write the following code:

```rust,norun
# use axum_socket_io::SocketIo;
# use std::net::SocketAddr;
#
pub async fn handle_socket(mut socket: SocketIo, addr: SocketAddr) {
    println!("A user connected: {addr:#?}");
    while let Ok(_ev) = socket.recv().await {
        // ...
    }
    println!("user disconnected: {addr:#?}");
}
```

In the `src/main.rs` file, write the following code:

```rust,ignore
mod socket;

# use axum::{
#     extract::ConnectInfo,
#     response::{Html, IntoResponse},
#     routing::get,
#     Router,
# };
# use axum_socket_io::SocketIoUpgrade;
# use std::{io, net::SocketAddr};
# use tokio::net::TcpListener;
#
#[tokio::main]
async fn main() -> io::Result<()> {
    let app = Router::new()
        .route("/", get(|| async { Html(include_str!("../index.html")) }))
        .route("/socket", get(ws_handler));

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
```

### Client Code

In your project directory, create an `index.html` file with the following
content:

```html
<script type="module">
    import { SocketIo } from "https://esm.sh/client-socket-io@0.1.0";
    const socket = new SocketIo("ws://127.0.0.1:3000/socket");
    await socket.connect();
    alert("Hello, World!")
</script>
```

### Running the Project

1. Build and run the Rust server:

```bash
cargo run
```

2. Go to `http://127.0.0.1:3000`, and you should see `Hello, World!`
