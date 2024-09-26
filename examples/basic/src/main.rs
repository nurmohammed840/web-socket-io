use axum::{
    extract::ConnectInfo,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use axum_socket_io::{Notifier, Procedure, SocketIo, SocketIoUpgrade};
use std::{collections::HashMap, io, net::SocketAddr, sync::LazyLock};
use tokio::{net::TcpListener, sync::mpsc::Sender};

#[tokio::main]
async fn main() -> io::Result<()> {
    let app = Router::new()
        .route("/", get(|| async { Html(include_str!("../index.html")) }))
        .route("/socket", get(ws_handler));

    let listener = TcpListener::bind("127.0.0.1:3000").await?;
    println!("listening on http://{}", listener.local_addr()?);

    LazyLock::force(&ROOM);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
}

async fn ws_handler(ws: SocketIoUpgrade, info: ConnectInfo<SocketAddr>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, info.0))
}

enum Action {
    Join { port: u16, notifier: Notifier },
    Broadcast(Box<[u8]>),
    Leave { port: u16 },
}

impl Action {
    async fn call(self) {
        ROOM.send(self).await.unwrap()
    }
}

static ROOM: LazyLock<Sender<Action>> = LazyLock::new(|| {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Action>(16);
    tokio::spawn(async move {
        let mut default_room: HashMap<u16, Notifier> = HashMap::new();

        while let Some(action) = rx.recv().await {
            match action {
                Action::Join { port, notifier } => {
                    for user in default_room.values() {
                        user.notify("new_user", port.to_string()).await.unwrap();
                    }
                    default_room.insert(port, notifier);
                }
                Action::Broadcast(msg) => {
                    for user in default_room.values() {
                        user.notify("chat", &msg).await.unwrap();
                    }
                }
                Action::Leave { port } => {
                    default_room.remove(&port);
                    for user in default_room.values() {
                        user.notify("user_disconnected", port.to_string())
                            .await
                            .unwrap();
                    }
                }
            }
        }
    });
    tx
});

async fn handle_socket(mut socket: SocketIo, addr: SocketAddr) {
    let port = addr.port();
    let notifier = socket.notifier();
    // susubscribe to default room.
    Action::Join { port, notifier }.call().await;

    while let Ok(ev) = socket.recv().await {
        match ev {
            Procedure::Call(req, res, _) => match req.method() {
                "myip" => res.send(addr.to_string()).await.unwrap(),
                _ => {}
            },
            Procedure::Notify(req) => match req.method() {
                "echo" => socket.notify("echo", req.data()).await.unwrap(),
                // broadcast to every users in default room.
                "chat_message" => Action::Broadcast(req.data().into()).call().await,
                _ => {}
            },
        }
    }
    Action::Leave { port }.call().await;
}
