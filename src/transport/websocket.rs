//! WebSocket transport built on [`tokio_tungstenite`].
//!
//! [`connect`] mirrors [`tokio_tungstenite::connect_async`] but transparently
//! routes the connection through the SOCKS5 proxy configured via
//! [`SOCKS5_PROXY`](super::proxy::PROXY_ENV_VAR) when one is set. The return
//! type is identical to `connect_async`, so it is a drop-in replacement.

use tokio::net::TcpStream;
use tokio_socks::IntoTargetAddr as _;
use tokio_socks::tcp::Socks5Stream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest as _;
use tokio_tungstenite::tungstenite::handshake::client::Response;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use super::proxy::ProxyConfig;
use crate::Result;
use crate::error::{Error, Kind};

/// WebSocket stream type returned by [`connect`].
///
/// Identical to the type produced by [`tokio_tungstenite::connect_async`]:
/// after the SOCKS5 handshake the proxied connection is a transparent TCP
/// tunnel, so it is wrapped in the same `MaybeTlsStream<TcpStream>`.
pub type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Establishes a WebSocket connection to `endpoint`, honoring the environment
/// proxy configuration.
///
/// When no proxy is configured this is exactly
/// [`tokio_tungstenite::connect_async`]. When `SOCKS5_PROXY` is set the
/// connection is tunneled through the proxy; with `socks5h://` the destination
/// host name is resolved by the proxy (no DNS leak), with `socks5://` it is
/// resolved locally first.
pub async fn connect(endpoint: &str) -> Result<(WsStream, Response)> {
    match ProxyConfig::from_env() {
        ProxyConfig::Direct => Ok(tokio_tungstenite::connect_async(endpoint).await?),
        ProxyConfig::Socks { url, remote_dns } => connect_socks(endpoint, &url, remote_dns).await,
    }
}

/// Connects to `endpoint` through the SOCKS5 proxy described by `proxy_url`.
async fn connect_socks(
    endpoint: &str,
    proxy_url: &str,
    remote_dns: bool,
) -> Result<(WsStream, Response)> {
    let url = url::Url::parse(endpoint)?;
    let host = url
        .host_str()
        .ok_or_else(|| Error::validation("WebSocket endpoint is missing a host"))?;
    let port = url
        .port_or_known_default()
        .ok_or_else(|| Error::validation("WebSocket endpoint is missing a port"))?;

    let proxy = url::Url::parse(proxy_url)?;
    let proxy_host = proxy
        .host_str()
        .ok_or_else(|| Error::validation("SOCKS5_PROXY is missing a host"))?;
    let proxy_port = proxy
        .port()
        .ok_or_else(|| Error::validation("SOCKS5_PROXY is missing a port"))?;
    let proxy_addr = format!("{proxy_host}:{proxy_port}");
    let proxy_user = proxy.username().to_owned();
    let proxy_pass = proxy.password().unwrap_or_default().to_owned();

    // `socks5h` defers DNS resolution to the proxy (no DNS leak); `socks5`
    // resolves the destination host locally before dialing the proxy.
    let target = if remote_dns {
        (host, port).into_target_addr().map_err(ws_error)?
    } else {
        let addr = tokio::net::lookup_host((host, port))
            .await
            .map_err(ws_error)?
            .next()
            .ok_or_else(|| Error::validation("unable to resolve WebSocket host"))?;
        addr.into_target_addr().map_err(ws_error)?
    };

    let socks = if proxy_user.is_empty() {
        Socks5Stream::connect(proxy_addr.as_str(), target).await
    } else {
        Socks5Stream::connect_with_password(proxy_addr.as_str(), target, &proxy_user, &proxy_pass)
            .await
    }
    .map_err(ws_error)?;

    // After a successful CONNECT, the SOCKS5 stream relays bytes transparently,
    // so the inner `TcpStream` is handed straight to the TLS/WebSocket
    // handshake — yielding the same stream type as `connect_async`.
    let tcp = socks.into_inner();

    let request = endpoint.into_client_request()?;
    Ok(tokio_tungstenite::client_async_tls(request, tcp).await?)
}

/// Wraps a transport-level error as a [`Kind::WebSocket`] [`Error`].
fn ws_error<E>(error: E) -> Error
where
    E: std::error::Error + Send + Sync + 'static,
{
    Error::with_source(Kind::WebSocket, error)
}
