use axum_socket_io::Notifier;
use std::{collections::HashMap, sync::LazyLock};
use tokio::sync::mpsc::{self, Sender};

pub enum Room {
    Join { id: u16, notifier: Notifier },
    Broadcast(&'static str, Box<[u8]>),
    Leave { id: u16 },
}

impl Room {
    pub async fn dispatch(self) {
        TASK.send(self).await.unwrap()
    }
}

pub static TASK: LazyLock<Sender<Room>> = LazyLock::new(|| {
    let (tx, mut rx) = mpsc::channel::<Room>(16);
    tokio::spawn(async move {
        let mut main_room = HashMap::new();

        while let Some(action) = rx.recv().await {
            match action {
                Room::Join { id, notifier } => {
                    main_room.insert(id, notifier);
                }
                Room::Broadcast(ev, msg) => {
                    for user in main_room.values() {
                        user.notify(ev, &msg).await.unwrap();
                    }
                }
                Room::Leave { id } => {
                    main_room.remove(&id);
                }
            }
        }
    });
    tx
});
