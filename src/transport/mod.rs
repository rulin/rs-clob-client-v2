//! Unified network transport layer with optional SOCKS5/SOCKS5H proxy support.
//!
//! Every outbound connection in the SDK is routed through this layer:
//!
//! - REST traffic via [`reqwest`] is created through [`http::build_client`] /
//!   [`http::client_builder`].
//! - WebSocket traffic via [`tokio_tungstenite`] is established through
//!   [`websocket::connect`].
//!
//! A single environment variable, [`proxy::PROXY_ENV_VAR`] (`SOCKS5_PROXY`),
//! configures the whole stack. When it is unset or empty every connection is
//! made directly, exactly like the upstream SDK. When it is set, all REST and
//! WebSocket connections — and any external [`reqwest`] client obtained from
//! [`http::global_client`] — transparently flow through the configured SOCKS5
//! proxy.
//!
//! See [`proxy::ProxyConfig`] for the supported values and their semantics.

pub mod http;
pub mod proxy;
#[cfg(any(feature = "ws", feature = "rtds"))]
pub mod websocket;

pub use proxy::ProxyConfig;
