//! HTTP transport built on [`reqwest`].
//!
//! Every [`reqwest::Client`] in the SDK is constructed here so the proxy
//! configured via [`SOCKS5_PROXY`](super::proxy::PROXY_ENV_VAR) is applied
//! uniformly. External project code can share the exact same configuration
//! through [`global_client`], so a single environment variable transparently
//! routes the whole HTTP stack.

use std::sync::OnceLock;

use super::proxy::ProxyConfig;
use crate::Result;

/// Returns a [`reqwest::ClientBuilder`] with the environment proxy applied.
///
/// Additional configuration (default headers, timeouts, ...) can be chained
/// before calling [`reqwest::ClientBuilder::build`]. When no proxy is
/// configured the builder is returned unmodified, matching the upstream SDK.
pub fn client_builder() -> Result<reqwest::ClientBuilder> {
    let builder = reqwest::Client::builder();

    match ProxyConfig::from_env() {
        ProxyConfig::Direct => Ok(builder),
        // `reqwest` understands both `socks5://` and `socks5h://` schemes
        // natively, so the URL is forwarded verbatim and the local vs. remote
        // DNS behavior is selected by the scheme itself.
        ProxyConfig::Socks { url, .. } => Ok(builder.proxy(reqwest::Proxy::all(url)?)),
    }
}

/// Builds a [`reqwest::Client`] with the environment proxy applied.
///
/// This is the single entry point that should be used in place of
/// `reqwest::Client::builder().build()` throughout the project.
pub fn build_client() -> Result<reqwest::Client> {
    Ok(client_builder()?.build()?)
}

/// Returns a process-wide shared [`reqwest::Client`] with the environment proxy
/// applied.
///
/// The client is constructed once, on first access, and reused afterwards.
/// External code that wants to inherit the SDK's proxy configuration without
/// re-configuring anything should send its requests through this client.
///
/// # Panics
///
/// Panics if the shared client cannot be built — for example if
/// [`SOCKS5_PROXY`](super::proxy::PROXY_ENV_VAR) is set to an invalid value.
#[must_use]
pub fn global_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| build_client().expect("failed to build the global HTTP client"))
}
