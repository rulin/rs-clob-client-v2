#![expect(
    clippy::module_name_repetitions,
    reason = "`ProxyConfig` is the public-facing name for this module's configuration type"
)]

//! Proxy configuration sourced from a single environment variable.
//!
//! The entire network stack is configured from [`PROXY_ENV_VAR`]
//! (`SOCKS5_PROXY`). See [`ProxyConfig::from_env`] for the accepted values.

/// Name of the environment variable used to configure the SOCKS proxy.
///
/// Accepted values:
///
/// - unset or empty — connect directly (identical to the upstream SDK).
/// - `socks5h://host:port` — SOCKS5 with DNS resolved by the proxy (no DNS
///   leak; recommended for Tor and private networks).
/// - `socks5://host:port` — SOCKS5 with DNS resolved locally.
///
/// Optional `user:password@` credentials embedded in the URL are honored by
/// both the REST and WebSocket transports.
pub const PROXY_ENV_VAR: &str = "SOCKS5_PROXY";

/// Network proxy configuration shared by every transport in the SDK.
///
/// Build one with [`ProxyConfig::from_env`].
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProxyConfig {
    /// Connect directly, without a proxy.
    Direct,
    /// Route all traffic through a SOCKS5 proxy.
    Socks {
        /// Full proxy URL, e.g. `socks5h://127.0.0.1:9050`.
        url: String,
        /// `true` for `socks5h://` (DNS resolved by the proxy), `false` for
        /// `socks5://` (DNS resolved locally).
        remote_dns: bool,
    },
}

impl ProxyConfig {
    /// Builds a [`ProxyConfig`] from the [`PROXY_ENV_VAR`] environment variable.
    ///
    /// # Panics
    ///
    /// Panics if [`PROXY_ENV_VAR`] is set to a non-empty value that does not
    /// begin with `socks5://` or `socks5h://`. An empty or unset variable
    /// yields [`ProxyConfig::Direct`].
    #[must_use]
    pub fn from_env() -> Self {
        Self::from_raw(&std::env::var(PROXY_ENV_VAR).unwrap_or_default())
    }

    /// Parses a raw proxy value (the contents of [`PROXY_ENV_VAR`]).
    ///
    /// Kept separate from [`ProxyConfig::from_env`] so the parsing logic can be
    /// exercised without mutating process-global environment state.
    fn from_raw(raw: &str) -> Self {
        let value = raw.trim();

        if value.is_empty() {
            return Self::Direct;
        }

        if value.starts_with("socks5h://") {
            return Self::Socks {
                url: value.to_owned(),
                remote_dns: true,
            };
        }

        if value.starts_with("socks5://") {
            return Self::Socks {
                url: value.to_owned(),
                remote_dns: false,
            };
        }

        panic!("{PROXY_ENV_VAR} must start with socks5:// or socks5h://");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_value_is_direct() {
        assert_eq!(ProxyConfig::from_raw(""), ProxyConfig::Direct);
    }

    #[test]
    fn whitespace_only_is_direct() {
        assert_eq!(ProxyConfig::from_raw("   \t\n"), ProxyConfig::Direct);
    }

    #[test]
    fn socks5h_enables_remote_dns() {
        assert_eq!(
            ProxyConfig::from_raw("socks5h://127.0.0.1:9050"),
            ProxyConfig::Socks {
                url: "socks5h://127.0.0.1:9050".to_owned(),
                remote_dns: true,
            }
        );
    }

    #[test]
    fn socks5_uses_local_dns() {
        assert_eq!(
            ProxyConfig::from_raw("socks5://127.0.0.1:9050"),
            ProxyConfig::Socks {
                url: "socks5://127.0.0.1:9050".to_owned(),
                remote_dns: false,
            }
        );
    }

    #[test]
    fn surrounding_whitespace_is_trimmed() {
        assert_eq!(
            ProxyConfig::from_raw("  socks5h://127.0.0.1:9050  "),
            ProxyConfig::Socks {
                url: "socks5h://127.0.0.1:9050".to_owned(),
                remote_dns: true,
            }
        );
    }

    #[test]
    #[should_panic(expected = "must start with socks5:// or socks5h://")]
    fn unknown_scheme_panics() {
        _ = ProxyConfig::from_raw("http://127.0.0.1:9050");
    }
}
