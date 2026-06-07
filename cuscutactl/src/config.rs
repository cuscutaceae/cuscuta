use clap::{Args, ValueEnum};
use serde::{Deserialize, Serialize};

/// Database connection configuration.
///
/// Works in two modes: `Legacy` (direct URLs) or `Kubernetes` (secrets, not yet implemented).
#[derive(Debug, Serialize, Deserialize, Clone, Args)]
pub struct Config {
    /// Working mode
    #[arg(short, long, value_enum, default_value_t = CommandMode::Legacy)]
    pub mode: CommandMode,

    #[command(flatten)]
    pub kubernetes: Option<Kubernetes>,

    #[command(flatten)]
    pub legacy: Option<Legacy>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Args)]
pub struct Kubernetes {
    /// Kubernetes namespace
    #[arg(long, name = "kube_namespace", default_value = "cuscuta")]
    pub namespace: String,

    /// Secret name
    #[arg(long, name = "kube_secret", default_value = "cuscuta-sql")]
    pub secret: String,

    /// Secret key for `PostgreSQL` URL
    #[arg(
        long,
        name = "kube_postgresql_key",
        default_value = "ACCOUNTS_SQL_ADDR"
    )]
    pub postgresql_key: String,

    /// Secret key for Redis URL
    #[arg(long, name = "kube_redis_key", default_value = "REDIS_ADDR")]
    pub redis_key: String,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Args)]
pub struct Legacy {
    /// `PostgreSQL` connection URL
    #[arg(long)]
    pub postgresql_url: Option<String>,

    /// `Redis` connection URL
    #[arg(long)]
    pub redis_url: Option<String>,
}

#[derive(
    Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Default, Serialize, Deserialize,
)]
pub enum CommandMode {
    /// Read database addresses from Kubernetes secrets (not yet implemented)
    #[default]
    #[value(alias = "k8s")]
    Kubernetes,

    /// Use URLs directly
    #[value(alias = "direct")]
    Legacy,
}

/// Configuration resolution errors.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Kubernetes mode is selected but not yet available.
    #[error("Kubernetes mode is not yet implemented; use --mode legacy")]
    KubernetesNotImplemented,

    /// No `--postgresql-url` was provided in legacy mode.
    #[error("--postgresql-url is required in legacy mode")]
    MissingPostgresqlUrl,

    /// No `--redis-url` was provided in legacy mode.
    #[error("--redis-url is required in legacy mode")]
    MissingRedisUrl,
}

impl Config {
    /// Extracts the `PostgreSQL` connection URL.
    ///
    /// # Errors
    ///
    /// Returns [`Error::KubernetesNotImplemented`] or [`Error::MissingPostgresqlUrl`].
    pub fn require_postgresql_url(&self) -> Result<String, Error> {
        match self.mode {
            CommandMode::Kubernetes => Err(Error::KubernetesNotImplemented),
            CommandMode::Legacy => {
                let legacy = self.legacy.as_ref().ok_or(Error::MissingPostgresqlUrl)?;
                let pg = legacy
                    .postgresql_url
                    .clone()
                    .ok_or(Error::MissingPostgresqlUrl)?;
                Ok(pg)
            }
        }
    }

    /// Extracts the Redis connection URL.
    ///
    /// # Errors
    ///
    /// Returns [`Error::KubernetesNotImplemented`] or [`Error::MissingRedisUrl`].
    pub fn resolve_redis_url(&self) -> Result<String, Error> {
        match self.mode {
            CommandMode::Kubernetes => Err(Error::KubernetesNotImplemented),
            CommandMode::Legacy => {
                let legacy = self.legacy.as_ref().ok_or(Error::MissingRedisUrl)?;
                let redis = legacy.redis_url.clone().ok_or(Error::MissingRedisUrl)?;
                Ok(redis)
            }
        }
    }
}
