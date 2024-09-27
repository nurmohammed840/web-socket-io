#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

/// Error types
pub mod error;
use error::{ConnClose, NotifyError, ReceiverClosed};
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

/// `SocketIo` manages WebSocket communication for handling RPC events.
/// 
/// It utilizes WebSocket  technology to facilitate real-time communication, providing mechanisms for sending requests 
/// and receiving responses.
/// 
/// Additionally, it supports RPC cancellation and timeout functionality, 
/// allowing for better control over ongoing operations.
/// 
/// The struct efficiently manages concurrent RPC events and notifies clients of relevant occurrences.
pub struct SocketIo {
    ws: WebSocket<Box<dyn AsyncRead + Send + Unpin + 'static>>,
    tx: Sender<Reply>,
    resetter: Resetter,
}

enum Reply {
    Ping(Box<[u8]>),
    Response(Box<[u8]>),
}

/// `Procedure` represents an RPC (Remote Procedure Call) or notification in the system.
pub enum Procedure {
    /// `Call` represents a RPC event
    Call(Request, Response, AbortController),

    /// `Notify` represents a one-way notification that includes only a request.
    Notify(Request),
}

/// `Notifier` is used to send notifications, Sends notifications where no response expected.
#[derive(Clone)]
pub struct Notifier {
    tx: Sender<Reply>,
}

async fn notify(tx: &Sender<Reply>, name: &str, data: &[u8]) -> Result<(), NotifyError> {
    let event_name = name.as_bytes();
    let event_name_len: u8 = event_name
        .len()
        .try_into()
        .map_err(|_| NotifyError::EventNameTooBig)?;

    let mut buf = Vec::with_capacity(5 + data.len());

    buf.push(1); // frame type
    buf.push(event_name_len);
    buf.extend_from_slice(event_name);
    buf.extend_from_slice(data);

    tx.send(Reply::Response(buf.into()))
        .await
        .map_err(|_| NotifyError::ReceiverClosed)
}

impl Notifier {
    /// Sends a notification with the given name and data.
    pub async fn notify(&self, name: &str, data: impl AsRef<[u8]>) -> Result<(), NotifyError> {
        notify(&self.tx, name, data.as_ref()).await
    }
}

impl SocketIo {
    /// Returns a `Notifier` for sending notifications.
    pub fn notifier(&self) -> Notifier {
        Notifier {
            tx: self.tx.clone(),
        }
    }

    /// Sends a notification with the given name and data.
    pub async fn notify(&mut self, name: &str, data: impl AsRef<[u8]>) -> Result<(), NotifyError> {
        notify(&self.tx, name, data.as_ref()).await
    }

    /// Creates a new `SocketIo` instance with the specified reader, writer, and buffer size.
    ///
    /// # Arguments
    ///
    /// * `reader` - The source for reading data.
    /// * `writer` - The destination for writing data.
    /// * `buffer` - The size of the buffer for the channel.
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

    /// Receives the next `Procedure` (either a rpc or notification).
    ///
    /// ## Connection State
    /// - Returns `io::ErrorKind::ConnectionReset` when an error event occurs.
    /// - Returns `io::ErrorKind::ConnectionAborted` when a close event is received.
    pub async fn recv(&mut self) -> io::Result<Procedure> {
        let mut buf = Vec::with_capacity(4096);
        let result = async {
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
                                if let ControlFlow::Break(p) =
                                    self.into_event(data).map_err(|err| {
                                        io::Error::new(io::ErrorKind::InvalidData, err)
                                    })?
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
                        return Err(io::Error::new(io::ErrorKind::ConnectionReset, err))
                    }
                    Event::Close { code, reason } => {
                        return Err(io::Error::new(
                            io::ErrorKind::ConnectionAborted,
                            ConnClose { code, reason },
                        ))
                    }
                }
            }
        }
        .await;
        if result.is_err() {
            for (_, reset_inner) in self.resetter.lock().unwrap().drain() {
                reset_inner.lock().unwrap().reset();
            }
        }
        result
    }

    fn into_event(&mut self, buf: Box<[u8]>) -> Result<ControlFlow<Procedure>, DynErr> {
        let reader = &mut &buf[..];
        let frame_type = get_slice(reader, 1)?[0];

        match frame_type {
            1 => {
                let method_len = validate_and_parse_utf8_rpc_name(reader)?;
                let data_offset = (buf.len() - reader.len()) as u16;
                Ok(ControlFlow::Break(Procedure::Notify(Request {
                    buf,
                    method_offset: 2,
                    method_len,
                    data_offset,
                })))
            }
            2 => {
                let id = parse_rpc_id(reader)?;
                let method_len = validate_and_parse_utf8_rpc_name(reader)?;
                let data_offset = (buf.len() - reader.len()) as u16;

                let reset = AbortController::new();
                self.resetter
                    .lock()
                    .unwrap()
                    .insert(id, reset.inner.clone());

                Ok(ControlFlow::Break(Procedure::Call(
                    Request {
                        buf,
                        method_offset: 6,
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
            3 => {
                let id = parse_rpc_id(reader)?;
                if let Some(reset_inner) = self.resetter.lock().unwrap().remove(&id) {
                    reset_inner.lock().unwrap().reset();
                }
                Ok(ControlFlow::Continue(()))
            }
            _ => Err("invalid frame".into()),
        }
    }
}

struct ResetInner {
    is_reset: bool,
    // todo: use `AtomicUsize` as state for both `is_reset` and `has_waker`
    // todo: use spinlock using `AtomicUsize` state ?
    waker: Option<std::task::Waker>,
}

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

type ResetShared = Arc<Mutex<ResetInner>>;

/// `AbortController` is a controller that allows you to monitor for a stream reset and
/// cancel an associated asynchronous task if the reset occurs.
pub struct AbortController {
    inner: ResetShared,
}

impl AbortController {
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(ResetInner::new())),
        }
    }

    /// Polls to be notified when the client resets this rpc.
    /// If the stream has not been reset. This returns `Poll::Pending`
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

    /// Awaits the stream reset event.
    pub async fn reset(&mut self) {
        std::future::poll_fn(|cx| self.poll_reset(cx)).await;
    }

    /// Executes a given asynchronous task and aborts it when stream is reset.
    ///
    /// ### Example
    ///
    /// ```rust
    /// controller.abort_on_reset(async {  }).await;
    /// ```
    pub async fn abort_on_reset(mut self, task: impl Future) {
        let mut task = std::pin::pin!(task);
        std::future::poll_fn(|cx| {
            if let Poll::Ready(()) = self.poll_reset(cx) {
                return Poll::Ready(());
            }
            task.as_mut().poll(cx).map(|_| ())
        })
        .await;
    }

    /// Spawns a new task that will be aborted if the stream is reset.
    ///
    /// This function spawns the given task in background, and automatically cancels
    /// the task if the stream reset event occurs.
    ///
    /// ### Example
    ///
    /// ```rust
    /// controller.spawn_and_abort_on_reset(async { ... });
    /// ```
    pub fn spawn_and_abort_on_reset<F>(self, task: F) -> tokio::task::JoinHandle<()>
    where
        F: Future + Send + 'static,
    {
        tokio::task::spawn(self.abort_on_reset(task))
    }
}

/// Represents an incoming rpc request.
#[derive(Debug)]
pub struct Request {
    buf: Box<[u8]>,
    method_offset: u8,
    method_len: u8,
    data_offset: u16,
}

/// Represents a response used to send the result of a rpc request.
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

/// Represents rpc response.
impl Response {
    /// Returns the ID of the rpc request.
    #[inline]
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Sends the response with the provided data.
    pub async fn send(self, data: impl AsRef<[u8]>) -> Result<(), ReceiverClosed> {
        let data = data.as_ref();
        let mut buf = Vec::with_capacity(5 + data.len());

        buf.push(4); // frame type
        buf.extend_from_slice(&self.id.to_be_bytes()); // call id
        buf.extend_from_slice(data);

        self.tx
            .send(Reply::Response(buf.into()))
            .await
            .map_err(|_| ReceiverClosed)
    }
}

impl Request {
    /// Returns the rpc method name.
    #[inline]
    pub fn method(&self) -> &str {
        unsafe {
            let offset = self.method_offset as usize;
            let length = self.method_len as usize;
            std::str::from_utf8_unchecked(&self.buf.get_unchecked(offset..(offset + length)))
        }
    }

    /// Returns the data payload of the request.
    #[inline]
    pub fn data(&self) -> &[u8] {
        &self.buf[self.data_offset.into()..]
    }
}

fn parse_rpc_id(reader: &mut &[u8]) -> Result<u32, &'static str> {
    let raw_id = get_slice(reader, 4)?;
    let id = u32::from_be_bytes(raw_id.try_into().unwrap());
    Ok(id)
}

fn validate_and_parse_utf8_rpc_name(reader: &mut &[u8]) -> Result<u8, DynErr> {
    let method_len = get_slice(reader, 1)?[0];
    std::str::from_utf8(get_slice(reader, method_len as usize)?)?;
    Ok(method_len)
}

fn get_slice<'de>(reader: &mut &'de [u8], len: usize) -> Result<&'de [u8], &'static str> {
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
