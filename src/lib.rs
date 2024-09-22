mod error;
mod frame;
use error::ConnClose;
use frame::{Notify, Request};
pub use web_socket;

use std::io;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::mpsc::Sender,
};
use web_socket::{DataType, Event, Stream, WebSocket};

pub(crate) type DynErr = Box<dyn std::error::Error + Send + Sync>;

pub struct SocketIo {
    ws: WebSocket<Box<dyn AsyncRead + Send + Unpin + 'static>>,
    reply: Sender<Reply>,
}

enum Reply {
    Ping(Box<[u8]>),
}

pub enum Procedure {
    Call(Request),
    Notify(Notify),
}

impl SocketIo {
    pub fn new<I, O>(reader: I, writer: O, buffer: usize) -> Self
    where
        I: Unpin + AsyncRead + Send + 'static,
        O: Unpin + AsyncWrite + Send + 'static,
    {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Reply>(buffer);
        let mut ws_writer = WebSocket::server(writer);
        tokio::spawn(async move {
            loop {
                while let Some(reply) = rx.recv().await {
                    let _ = match reply {
                        Reply::Ping(data) => ws_writer.send_pong(data).await,
                    };
                }
            }
        });
        Self {
            ws: WebSocket::server(Box::new(reader)),
            reply: tx,
        }
    }

    pub async fn recv(&mut self) -> io::Result<Procedure> {
        let mut buf = Vec::with_capacity(4096);
        loop {
            match self.ws.recv().await? {
                Event::Data { ty, data } => match ty {
                    DataType::Complete(_) => {
                        return into_event(data);
                    }
                    DataType::Stream(stream) => {
                        buf.extend_from_slice(&data);
                        if let Stream::End(_) = stream {
                            return into_event(buf.into());
                        }
                    }
                },
                Event::Ping(data) => {
                    let _ = self.reply.send(Reply::Ping(data)).await;
                }
                Event::Pong(_) => {}
                Event::Error(err) => {
                    return Err(io::Error::new(io::ErrorKind::ConnectionReset, err));
                }
                Event::Close { code, reason } => {
                    return Err(io::Error::new(
                        io::ErrorKind::ConnectionAborted,
                        ConnClose { code, reason },
                    ));
                }
            }
        }
    }
}

fn into_event(data: Box<[u8]>) -> io::Result<Procedure> {
    let req =
        Request::parse(data).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

    if req.id() == 0 {
        Ok(Procedure::Notify(req.into()))
    } else {
        Ok(Procedure::Call(req))
    }
}
