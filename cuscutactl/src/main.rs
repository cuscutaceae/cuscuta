//! CLI management tool for [cuscuta](https://github.com/cuscutaceae/cuscuta) clusters.
//!
//! `cuscutactl` provides direct administrative access to the `PostgreSQL` and `Redis`
//! databases backing a cuscuta deployment:
//!
//! - **`PostgreSQL`**: account CRUD, rating adjustments, manual lease release
//! - **`Redis`**: job queue inspection, result retrieval
//!
//! # Quick examples
//!
//! ```shell
//! # Health check (Kubernetes mode)
//! cuscutactl --mode kubernetes --kube-namespace cuscuta doctor
//!
//! # Health check (Legacy mode)
//! cuscutactl --postgresql-url "..." --redis-url "..." doctor
//!
//! # Add accounts in batch
//! cat accounts.txt | cuscutactl --postgresql-url "..." accounts row add --stdin
//!
//! # Inspect job results
//! cuscutactl --redis-url "..." jobs result --code 123456789 --print-detail
//! ```
//!
//! # Connection modes
//!
//! Two modes are available via `--mode` (or `-m`):
//!
//! | Mode         | Alias    | Description                                             |
//! |--------------|----------|---------------------------------------------------------|
//! | `legacy`     | `direct` | Direct URLs via `--postgresql-url` / `--redis-url`      |
//! | `kubernetes` | `k8s`    | Read URLs from cluster Secret via kubectl port-forward  |
//!
//! `legacy` is the default. `kubernetes` mode reads database URLs from the cluster
//! Secret (`cuscuta-secret` by default) and opens a local `kubectl port-forward` tunnel.

use std::pin::Pin;

use anyhow::bail;
use clap::Parser;

use crate::{
    command::{
        Cli, SubCommandAccounts, SubCommandAccountsRate, SubCommandAccountsRow, SubCommandJobs,
        SubCommands,
    },
    config::{CommandMode, Kubernetes},
    kube::k8s_port_forward,
};

mod accounts;
mod command;
mod config;
mod doctor;
mod jobs;
mod kube;
mod stats;
mod url;

trait Handler {
    fn get_url(&self) -> String;
    fn kill(&mut self) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;
}

impl Handler for String {
    fn get_url(&self) -> String {
        self.clone()
    }

    fn kill(&mut self) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(std::future::ready(()))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    type DynHandlerResult = anyhow::Result<Box<dyn Handler>>;
    let cli = Cli::parse();
    let (postgresql_handle, redis_handle): (DynHandlerResult, DynHandlerResult) =
        match &cli.config.mode {
            CommandMode::Kubernetes => {
                let Kubernetes {
                    namespace,
                    secret,
                    postgresql_key,
                    redis_key,
                    cluster_domain,
                    postgresql_port,
                    redis_port,
                    postgresql_forward_port,
                    redis_forward_port,
                } = &cli.config.kubernetes;
                (
                    k8s_port_forward(
                        namespace,
                        secret,
                        postgresql_key,
                        cluster_domain,
                        postgresql_forward_port,
                        postgresql_port,
                    )
                    .await
                    .map(|it| Box::new(it) as Box<dyn Handler>),
                    k8s_port_forward(
                        namespace,
                        secret,
                        redis_key,
                        cluster_domain,
                        redis_forward_port,
                        redis_port,
                    )
                    .await
                    .map(|it| Box::new(it) as Box<dyn Handler>),
                )
            }
            CommandMode::Legacy => (
                Box::new(cli.config.resolve_legacy_postgresql_url())
                    .map_err(anyhow::Error::from)
                    .map(|it| Box::new(it) as Box<dyn Handler>),
                cli.config
                    .resolve_legacy_redis_url()
                    .map_err(anyhow::Error::from)
                    .map(|it| Box::new(it) as Box<dyn Handler>),
            ),
        };
    let pg_url = postgresql_handle
        .as_ref()
        .map(|it| it.get_url())
        .map_err(std::string::ToString::to_string);
    let redis_url = redis_handle
        .as_ref()
        .map(|it| it.get_url())
        .map_err(std::string::ToString::to_string);
    let _ = run_command(&cli, pg_url, redis_url).await;
    if let Ok(mut postgresql_handle) = postgresql_handle {
        postgresql_handle.kill().await;
    }
    if let Ok(mut redis_handle) = redis_handle {
        redis_handle.kill().await;
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
async fn run_command(
    cli: &Cli,
    pg_url: Result<String, String>,
    redis_url: Result<String, String>,
) -> anyhow::Result<()> {
    match &cli.sub_command {
        SubCommands::Doctor => {
            let redis_url = redis_url.as_ref().map_or_else(
                |_| {
                    eprintln!("warning: no redis url specified, redis check will be skipped");
                    eprintln!("Hint: use --mode legacy --redis-url <URL>");
                    None
                },
                Some,
            );
            let pg_url = pg_url.as_ref().map_or_else(
                |_| {
                    eprintln!(
                        "warning: no postgresql url specified, account check will be skipped"
                    );
                    eprintln!("Hint: use --mode legacy --postgresql-url <URL>");
                    None
                },
                Some,
            );
            doctor::run(pg_url, redis_url).await?;
        }
        SubCommands::Jobs { command } => {
            let redis_url = match redis_url {
                Ok(x) => x,
                Err(e) => {
                    eprintln!("Configuration error: {e:?}");
                    eprintln!("Hint: use --mode legacy --redis-url <URL>");
                    bail!(e);
                }
            };
            match command {
                SubCommandJobs::Status { max_count } => jobs::status(&redis_url, *max_count)?,
                SubCommandJobs::Find { code, max_count } => {
                    jobs::find(&redis_url, code, *max_count)?;
                }
                SubCommandJobs::Result {
                    code,
                    max_count,
                    print_detail,
                } => jobs::result(&redis_url, code, *max_count, *print_detail)?,
            }
        }

        SubCommands::Accounts { command } => {
            let pg_url = match pg_url {
                Ok(x) => x,
                Err(e) => {
                    eprintln!("Configuration error: {e:?}");
                    eprintln!("Hint: use --mode legacy --postgresql-url <URL>");
                    bail!(e);
                }
            };
            match command {
                SubCommandAccounts::Status { max_count } => {
                    accounts::status(&pg_url, *max_count).await?;
                }
                SubCommandAccounts::Row { command } => match command {
                    SubCommandAccountsRow::Add {
                        email,
                        password,
                        stdin,
                    } => {
                        accounts::row_add(&pg_url, email.clone(), password.clone(), *stdin).await?;
                    }
                    SubCommandAccountsRow::Remove { id } => {
                        accounts::row_remove(&pg_url, *id).await?;
                    }
                    SubCommandAccountsRow::Query { id } => {
                        accounts::row_query(&pg_url, *id).await?;
                    }
                },
                SubCommandAccounts::Rate { command, id } => match command {
                    SubCommandAccountsRate::Set { value, delta } => {
                        accounts::rate_set(&pg_url, *id, *value, *delta).await?;
                    }
                    SubCommandAccountsRate::Query => accounts::rate_query(&pg_url, *id).await?,
                },
                SubCommandAccounts::Release { id, force } => {
                    accounts::release(&pg_url, *id, *force).await?;
                }
            }
        }
        SubCommands::Stats { command } => {
            let redis_url = match redis_url {
                Ok(x) => x,
                Err(e) => {
                    eprintln!("Configuration error: {e:?}");
                    eprintln!("Hint: use --mode legacy --redis-url <URL>");
                    bail!(e);
                }
            };
            match command {
                command::SubCommandStats::Worker {} => stats::worker(&redis_url)?,
                command::SubCommandStats::Event { show_level, limit } => {
                    stats::event(&redis_url, *show_level, *limit)?;
                }
            }
        }
    }
    Ok(())
}
