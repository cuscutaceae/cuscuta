use clap::{Args, ValueEnum};
use serde::{Deserialize, Serialize};

/// Database connection configuration.
///
#[derive(Debug, Serialize, Deserialize, Clone, Args)]
pub struct Config {
    /// Working mode
    #[arg(short, long, value_enum, default_value_t = CommandMode::Legacy)]
    pub mode: CommandMode,

    #[command(flatten)]
    pub kubernetes: Kubernetes,

    #[command(flatten)]
    pub legacy: Option<Legacy>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Args)]
pub struct Kubernetes {
    /// Kubernetes namespace
    #[arg(long, name = "kube_namespace", default_value = "cuscuta")]
    pub namespace: String,

    /// Secret name
    #[arg(long, name = "kube_secret", default_value = "cuscuta-secret")]
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

    /// Cluster domain
    #[arg(long, name = "kube_cluster_domain", default_value = "cluster.local")]
    pub cluster_domain: String,

    /// `PostgreSQL` port from container
    #[arg(long, name = "kube_postgresql_port", default_value_t = 5432)]
    pub postgresql_port: u16,

    /// Redis port from container
    #[arg(long, name = "kube_redis_port", default_value_t = 6379)]
    pub redis_port: u16,

    /// Forwarded `PostgreSQL` port to host
    #[arg(long, name = "kube_postgresql_forward_port", default_value_t = 50001)]
    pub postgresql_forward_port: u16,

    /// Forwarded Redis port to host
    #[arg(long, name = "kube_redis_forward_port", default_value_t = 50002)]
    pub redis_forward_port: u16,
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
    /// Read database addresses from Kubernetes secrets
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
    /// No --legacy sub-options were provided in legacy mode.
    #[error("--mode legacy requires --postgresql-url and/or --redis-url")]
    LegacyOptionMissing,

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
    /// Returns [`Error::LegacyOptionMissing`] or [`Error::MissingPostgresqlUrl`].
    pub fn resolve_legacy_postgresql_url(&self) -> Result<String, Error> {
        let pg = self
            .legacy
            .clone()
            .ok_or(Error::LegacyOptionMissing)?
            .postgresql_url
            .ok_or(Error::MissingPostgresqlUrl)?;
        Ok(pg)
    }

    /// Extracts the Redis connection URL.
    ///
    /// # Errors
    ///
    /// Returns [`Error::LegacyOptionMissing`] or [`Error::MissingRedisUrl`].
    pub fn resolve_legacy_redis_url(&self) -> Result<String, Error> {
        let pg = self
            .legacy
            .clone()
            .ok_or(Error::LegacyOptionMissing)?
            .redis_url
            .ok_or(Error::MissingRedisUrl)?;
        Ok(pg)
    }
}
