//! **This crate is mostly AI-generated**
//!
//! CLI management tool for [cuscuta](https://github.com/cuscutaceae/cuscuta) clusters.
//!
//! `cuscutactl` provides direct administrative access to the `PostgreSQL` and Redis
//! databases backing a cuscuta deployment:
//!
//! - **`PostgreSQL`**: account CRUD, rating adjustments, manual lease release
//! - **`Redis`**: job queue inspection, result retrieval
//!
//! # Quick examples
//!
//! ```shell
//! # Health check
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
//! | Mode         | Alias   | Description                                        |
//! |--------------|---------|----------------------------------------------------|
//! | `legacy`     | `direct`| Reads URLs from `--postgresql-url` / `--redis-url` |
//! | `kubernetes` | `k8s`   | Reads URLs from Kubernetes secrets (not yet implemented) |
//!
//! `legacy` is the default.

#![deny(clippy::pedantic)]
#![deny(missing_docs)]

use clap::Parser;

use crate::command::{
    Cli, SubCommandAccounts, SubCommandAccountsRate, SubCommandAccountsRow, SubCommandJobs,
    SubCommands,
};

mod accounts;
mod command;
mod config;
mod doctor;
mod jobs;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let pg_url = cli.config.require_postgresql_url();
    let redis_url = cli.config.resolve_redis_url();
    match cli.sub_command {
        SubCommands::Doctor => {
            let redis_url = if let Ok(ref x) = redis_url {
                Some(x)
            } else {
                eprintln!("warning: no redis url specified, redis check will be skipped");
                eprintln!("Hint: use --mode legacy --redis-url <URL>");
                None
            };
            let pg_url = if let Ok(ref x) = pg_url {
                Some(x)
            } else {
                eprintln!("warning: no postgresql url specified, account check will be skipped");
                eprintln!("Hint: use --mode legacy --postgresql-url <URL>");
                None
            };
            doctor::run(pg_url, redis_url).await?;
        }

        SubCommands::Jobs { command } => {
            let redis_url = match redis_url {
                Ok(x) => x,
                Err(e) => {
                    eprintln!("Configuration error: {e:?}");
                    eprintln!("Hint: use --mode legacy --redis-url <URL>");
                    std::process::exit(1);
                }
            };
            match command {
                SubCommandJobs::Status { max_count } => jobs::status(&redis_url, max_count)?,
                SubCommandJobs::Find { code, max_count } => {
                    jobs::find(&redis_url, &code, max_count)?;
                }
                SubCommandJobs::Result {
                    code,
                    max_count,
                    print_detail,
                } => jobs::result(&redis_url, &code, max_count, print_detail)?,
            }
        }

        SubCommands::Accounts { command } => {
            let pg_url = match pg_url {
                Ok(x) => x,
                Err(e) => {
                    eprintln!("Configuration error: {e:?}");
                    eprintln!("Hint: use --mode legacy --postgresql-url <URL>");
                    std::process::exit(1);
                }
            };
            match command {
                SubCommandAccounts::Status { max_count } => {
                    accounts::status(&pg_url, max_count).await?;
                }
                SubCommandAccounts::Row { command } => match command {
                    SubCommandAccountsRow::Add {
                        email,
                        password,
                        stdin,
                    } => {
                        accounts::row_add(&pg_url, email, password, stdin).await?;
                    }
                    SubCommandAccountsRow::Remove { id } => {
                        accounts::row_remove(&pg_url, id).await?;
                    }
                    SubCommandAccountsRow::Query { id } => accounts::row_query(&pg_url, id).await?,
                },
                SubCommandAccounts::Rate { command, id } => match command {
                    SubCommandAccountsRate::Set { value, delta } => {
                        accounts::rate_set(&pg_url, id, value, delta).await?;
                    }
                    SubCommandAccountsRate::Query => accounts::rate_query(&pg_url, id).await?,
                },
                SubCommandAccounts::Release { id, force } => {
                    accounts::release(&pg_url, id, force).await?;
                }
            }
        }
    }

    Ok(())
}
