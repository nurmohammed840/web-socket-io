use axum_socket_io::Notifier;
use std::{collections::HashMap, sync::LazyLock};
use tokio::sync::mpsc::{self, Sender};

pub enum Action {
    Join { id: u16, notifier: Notifier },
    Broadcast(Box<[u8]>),
    Leave { id: u16 },
}

impl Action {
    pub async fn dispatch(self) {
        TASK.send(self).await.unwrap()
    }
}

pub static TASK: LazyLock<Sender<Action>> = LazyLock::new(|| {
    let (tx, mut rx) = mpsc::channel::<Action>(16);
    tokio::spawn(async move {
        let mut main_room = HashMap::new();

        while let Some(action) = rx.recv().await {
            match action {
                Action::Join { id, notifier } => {
                    main_room.insert(id, notifier);
                }
                Action::Broadcast(msg) => {
                    for user in main_room.values() {
                        user.notify("message", &msg).await.unwrap();
                    }
                }
                Action::Leave { id } => {
                    main_room.remove(&id);
                }
            }
        }
    });
    tx
});
