pub mod error;
use error::{ConnClose, EmitError, ReceiverClosed};
pub use web_socket;

use std::{
    collections::HashMap,
    future::Future,
    io,
    ops::ControlFlow,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::mpsc::Sender,
};
use web_socket::{DataType, Event, Stream, WebSocket};

pub(crate) type DynErr = Box<dyn std::error::Error + Send + Sync>;

type Resetter = Arc<Mutex<HashMap<u32, ResetShared>>>;

pub struct SocketIo {
    ws: WebSocket<Box<dyn AsyncRead + Send + Unpin + 'static>>,
    tx: Sender<Reply>,
    resetter: Resetter,
}

enum Reply {
    Ping(Box<[u8]>),
    Response(Box<[u8]>),
}

pub enum Procedure {
    Call(Request, Response, Reset),
    Notify(Request),
}

#[derive(Clone)]
pub struct Emitter {
    tx: Sender<Reply>,
}

async fn emit(tx: &Sender<Reply>, name: &str, data: &[u8]) -> Result<(), EmitError> {
    let raw_name = name.as_bytes();
    let method_name_len: u8 = raw_name
        .len()
        .try_into()
        .map_err(|_| EmitError::EventNameTooBig)?;

    let mut buf = Vec::with_capacity(5 + data.len());

    buf.push(2); // frame type
    buf.push(method_name_len);
    buf.extend_from_slice(raw_name);
    buf.extend_from_slice(data);

    tx.send(Reply::Response(buf.into()))
        .await
        .map_err(|_| EmitError::ReceiverClosed)
}

impl Emitter {
    pub async fn emit(&self, name: &str, data: impl AsRef<[u8]>) -> Result<(), EmitError> {
        emit(&self.tx, name, data.as_ref()).await
    }
}

impl SocketIo {
    pub fn emitter(&self) -> Emitter {
        Emitter {
            tx: self.tx.clone(),
        }
    }

    pub async fn emit(&mut self, name: &str, data: impl AsRef<[u8]>) -> Result<(), EmitError> {
        emit(&self.tx, name, data.as_ref()).await
    }

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
                    let o = match reply {
                        Reply::Ping(data) => ws_writer.send_pong(data).await,
                        Reply::Response(data) => ws_writer.send(&data[..]).await,
                    };
                    if o.is_err() {
                        break;
                    }
                }
            }
        });
        Self {
            ws: WebSocket::server(Box::new(reader)),
            tx,
            resetter: Default::default(),
        }
    }

    pub async fn recv(&mut self) -> io::Result<Procedure> {
        let mut buf = Vec::with_capacity(4096);
        loop {
            match self.ws.recv().await? {
                Event::Data { ty, data } => match ty {
                    DataType::Complete(_) => {
                        if let ControlFlow::Break(p) = self
                            .into_event(data)
                            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?
                        {
                            return Ok(p);
                        }
                    }
                    DataType::Stream(stream) => {
                        buf.extend_from_slice(&data);
                        if let Stream::End(_) = stream {
                            if let ControlFlow::Break(p) = self
                                .into_event(data)
                                .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?
                            {
                                return Ok(p);
                            }
                        }
                    }
                },
                Event::Ping(data) => {
                    let _ = self.tx.send(Reply::Ping(data)).await;
                }
                Event::Pong(_) => {}
                Event::Error(err) => {
                    self.drop_all_calls();
                    return Err(io::Error::new(io::ErrorKind::ConnectionReset, err));
                }
                Event::Close { code, reason } => {
                    self.drop_all_calls();
                    return Err(io::Error::new(
                        io::ErrorKind::ConnectionAborted,
                        ConnClose { code, reason },
                    ));
                }
            }
        }
    }

    fn into_event(&mut self, buf: Box<[u8]>) -> Result<ControlFlow<Procedure>, DynErr> {
        let reader = &mut &buf[..];
        let frame_type = get_slice(reader, 1)?[0];

        match frame_type {
            1 => {
                let method_len = parse_fn_name_len(reader)?;
                let id = parse_call_id(reader)?;
                let data_offset = (buf.len() - reader.len()) as u16;

                let reset = Reset::new();
                self.resetter
                    .lock()
                    .unwrap()
                    .insert(id, reset.inner.clone());

                Ok(ControlFlow::Break(Procedure::Call(
                    Request {
                        buf,
                        method_len,
                        data_offset,
                    },
                    Response {
                        id,
                        tx: self.tx.clone(),
                        resetter: self.resetter.clone(),
                    },
                    reset,
                )))
            }
            2 => {
                let method_len = parse_fn_name_len(reader)?;
                let data_offset = (buf.len() - reader.len()) as u16;
                Ok(ControlFlow::Break(Procedure::Notify(Request {
                    buf,
                    method_len,
                    data_offset,
                })))
            }
            3 => {
                let id = parse_call_id(reader)?;
                if let Some(reset_inner) = self.resetter.lock().unwrap().remove(&id) {
                    reset_inner.lock().unwrap().reset();
                }
                Ok(ControlFlow::Continue(()))
            }
            _ => Err("invalid frame".into()),
        }
    }

    fn drop_all_calls(&mut self) {
        for (_, reset_inner) in self.resetter.lock().unwrap().drain() {
            reset_inner.lock().unwrap().reset();
        }
    }
}

struct ResetInner {
    is_reset: bool,
    // todo: use `AtomicUsize` as state for both `is_reset` and `has_waker`
    // todo: use spinlock using `AtomicUsize` state ?
    waker: Option<std::task::Waker>,
}
type ResetShared = Arc<Mutex<ResetInner>>;

impl ResetInner {
    fn new() -> Self {
        Self {
            is_reset: false,
            waker: None,
        }
    }

    fn reset(&mut self) {
        self.is_reset = true;
        if let Some(waker) = &self.waker {
            waker.wake_by_ref();
        }
    }
}

pub struct Reset {
    inner: ResetShared,
}

impl Reset {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(ResetInner::new())),
        }
    }

    pub fn poll_reset(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        let mut inner = self.inner.lock().unwrap();
        if inner.is_reset {
            return Poll::Ready(());
        }
        match inner.waker.as_mut() {
            Some(w) => w.clone_from(cx.waker()),
            None => inner.waker = Some(cx.waker().clone()),
        }
        drop(inner);
        Poll::Pending
    }

    pub async fn reset_task(&mut self, task: impl Future<Output = ()>) {
        let mut task = std::pin::pin!(task);
        std::future::poll_fn(|cx| {
            if let Poll::Ready(()) = self.poll_reset(cx) {
                return Poll::Ready(());
            }
            task.as_mut().poll(cx)
        })
        .await;
    }

    pub async fn on_reset(&mut self) {
        std::future::poll_fn(|cx| self.poll_reset(cx)).await;
    }
}

#[derive(Debug)]
pub struct Request {
    buf: Box<[u8]>,
    method_len: u8,
    data_offset: u16,
}

pub struct Response {
    id: u32,
    tx: Sender<Reply>,
    resetter: Resetter,
}

impl Drop for Response {
    fn drop(&mut self) {
        self.resetter.lock().unwrap().remove(&self.id);
    }
}

impl Response {
    #[inline]
    pub fn id(&self) -> u32 {
        self.id
    }

    pub async fn response(self, data: impl AsRef<[u8]>) -> Result<(), ReceiverClosed> {
        let data = data.as_ref();
        let mut buf = Vec::with_capacity(5 + data.len());

        buf.push(1); // frame type
        buf.extend_from_slice(&self.id.to_be_bytes()); // call id
        buf.extend_from_slice(data);

        self.tx
            .send(Reply::Response(buf.into()))
            .await
            .map_err(|_| ReceiverClosed)
    }
}

impl Request {
    #[inline]
    pub fn method(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(&self.buf[2..(self.method_len as usize) + 2]) }
    }

    #[inline]
    pub fn data(&self) -> &[u8] {
        &self.buf[self.data_offset.into()..]
    }
}

fn parse_call_id(reader: &mut &[u8]) -> Result<u32, &'static str> {
    let raw_id = get_slice(reader, 4)?;
    let id = u32::from_be_bytes(raw_id.try_into().unwrap());
    Ok(id)
}

fn parse_fn_name_len(reader: &mut &[u8]) -> Result<u8, DynErr> {
    let method_len = get_slice(reader, 1)?[0];
    std::str::from_utf8(get_slice(reader, method_len as usize)?)?;
    Ok(method_len)
}

pub fn get_slice<'de>(reader: &mut &'de [u8], len: usize) -> Result<&'de [u8], &'static str> {
    if len <= reader.len() {
        unsafe {
            let slice = reader.get_unchecked(..len);
            *reader = reader.get_unchecked(len..);
            Ok(slice)
        }
    } else {
        Err("insufficient bytes")
    }
}
