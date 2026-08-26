//! intueina-config — one TOML, strict, fail-closed (F-001, F-002).
//! Unknown keys abort boot naming the key. Port constant exported for docs.

use serde::{Deserialize, Serialize};

/// Single source of truth for the port — referenced by docs, server, and tests.
/// F-002: one constant, defined once, generated into every document that names it.
pub const PORT: u16 = 9124;
pub const BIND: &str = "127.0.0.1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_bind")]
    pub bind: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_data_dir")]
    pub data_dir: String,
    #[serde(default = "default_review_interval")]
    pub review_interval_days: u32,
    #[serde(default = "default_mcp_transport")]
    pub mcp_transport: String,
}

fn default_port() -> u16 { PORT }
fn default_bind() -> String { BIND.into() }
fn default_model() -> String { "Snowflake/snowflake-arctic-embed-m".into() }
fn default_data_dir() -> String { "~/.intueina".into() }
fn default_review_interval() -> u32 { 3 }
fn default_mcp_transport() -> String { "proxy".into() }

impl Default for Config {
    fn default() -> Self {
        Self {
            port: PORT,
            bind: BIND.into(),
            model: default_model(),
            data_dir: default_data_dir(),
            review_interval_days: 3,
            mcp_transport: "proxy".into(),
        }
    }
}

impl Config {
    pub fn from_toml_str(s: &str) -> anyhow::Result<Self> {
        toml::from_str(s).map_err(|e| anyhow::anyhow!("config parse failed: {e}"))
    }
    pub fn from_file(path: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        let s = std::fs::read_to_string(path)?;
        Self::from_toml_str(&s)
    }
    pub fn addr(&self) -> String { format!("{}:{}", self.bind, self.port) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_key_fails_closed() {
        let err = Config::from_toml_str("port = 9124\nunknown_key = 1").unwrap_err();
        assert!(err.to_string().contains("unknown"), "got: {err}");
    }

    #[test]
    fn defaults_when_empty() {
        let c = Config::from_toml_str("").unwrap();
        assert_eq!(c.port, PORT);
        assert_eq!(c.bind, BIND);
    }

    #[test]
    fn single_port_constant() {
        // F-002: docs must cite this constant, not a literal
        assert_eq!(Config::default().port, PORT);
        assert_eq!(Config::default().addr(), format!("{BIND}:{PORT}"));
    }
}
