#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

use axum::{
    async_trait,
    body::Bytes,
    extract::FromRequestParts,
    http::{header, request::Parts, HeaderMap, HeaderName, HeaderValue, Method, StatusCode},
};
use hyper_util::rt::TokioIo;
use std::future::Future;

pub use web_socket_io::*;

/// Extractor for establishing `SocketIo` connections.
pub struct SocketIoUpgrade {
    sec_websocket_key: HeaderValue,
    on_upgrade: hyper::upgrade::OnUpgrade,
}

impl SocketIoUpgrade {
    /// Finalize upgrading the connection and call the provided callback with `SocketIo` instance.
    ///
    /// ## Arguments
    ///
    /// * `buffer` - The size of the buffer to be used in the `SocketIo` instance.
    /// * `callback` - A function that will be called with the upgraded `SocketIo` instance.
    pub fn on_upgrade<C, Fut>(self, buffer: usize, callback: C) -> axum::response::Response
    where
        C: FnOnce(SocketIo) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        tokio::spawn(async move {
            if let Ok(upgraded) = self.on_upgrade.await {
                let (reader, writer) = tokio::io::split(TokioIo::new(upgraded));
                callback(SocketIo::new(reader, writer, buffer)).await;
            }
        });

        static H_UPGRADE: HeaderValue = HeaderValue::from_static("upgrade");
        static H_WEBSOCKET: HeaderValue = HeaderValue::from_static("websocket");
        static H_WS_PROTOCOL: HeaderValue = HeaderValue::from_static("websocket.io-rpc-v0.1");

        axum::response::Response::builder()
            .status(StatusCode::SWITCHING_PROTOCOLS)
            .header(header::CONNECTION, H_UPGRADE.clone())
            .header(header::UPGRADE, H_WEBSOCKET.clone())
            .header(header::SEC_WEBSOCKET_PROTOCOL, H_WS_PROTOCOL.clone())
            .header(
                header::SEC_WEBSOCKET_ACCEPT,
                sign(self.sec_websocket_key.as_bytes()),
            )
            .body(axum::body::Body::empty())
            .unwrap()
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for SocketIoUpgrade
where
    S: Send + Sync,
{
    type Rejection = ();

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        if parts.method != Method::GET {
            return Err(());
        }
        if !header_contains(&parts.headers, header::CONNECTION, "upgrade") {
            return Err(());
        }
        if !header_eq(&parts.headers, header::UPGRADE, "websocket") {
            return Err(());
        }
        if !header_eq(&parts.headers, header::SEC_WEBSOCKET_VERSION, "13") {
            return Err(());
        }
        if !header_eq(
            &parts.headers,
            header::SEC_WEBSOCKET_PROTOCOL,
            "websocket.io-rpc-v0.1",
        ) {
            return Err(());
        }
        Ok(Self {
            sec_websocket_key: parts
                .headers
                .get(header::SEC_WEBSOCKET_KEY)
                .ok_or(())?
                .clone(),

            on_upgrade: parts
                .extensions
                .remove::<hyper::upgrade::OnUpgrade>()
                .ok_or(())?,
        })
    }
}

fn sign(key: &[u8]) -> HeaderValue {
    use base64::engine::Engine as _;
    use sha1::{Digest, Sha1};

    let mut sha1 = Sha1::default();
    sha1.update(key);
    sha1.update(&b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11"[..]);
    let b64 = Bytes::from(base64::engine::general_purpose::STANDARD.encode(sha1.finalize()));
    HeaderValue::from_maybe_shared(b64).expect("base64 is a valid value")
}

fn header_eq(headers: &HeaderMap, key: HeaderName, value: &'static str) -> bool {
    if let Some(header) = headers.get(&key) {
        header.as_bytes().eq_ignore_ascii_case(value.as_bytes())
    } else {
        false
    }
}

fn header_contains(headers: &HeaderMap, key: HeaderName, value: &'static str) -> bool {
    let header = if let Some(header) = headers.get(&key) {
        header
    } else {
        return false;
    };
    if let Ok(header) = std::str::from_utf8(header.as_bytes()) {
        header.to_ascii_lowercase().contains(value)
    } else {
        false
    }
}
