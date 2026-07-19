use clap::{Parser, Subcommand, ValueEnum};
use cuscuta_common::db::log::WorkerEventType;
use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(flatten)]
    pub config: Config,

    #[command(subcommand)]
    pub sub_command: SubCommands,
}

#[derive(Debug, Subcommand)]
pub enum SubCommands {
    /// Check cuscuta database health
    #[command(visible_alias = "check")]
    Doctor,

    /// Job queue operations
    #[command(visible_alias = "job")]
    Jobs {
        #[command(subcommand)]
        command: SubCommandJobs,
    },

    /// Account operations
    #[command(visible_alias = "account")]
    Accounts {
        #[command(subcommand)]
        command: SubCommandAccounts,
    },

    /// Worker stat query
    #[command(visible_alias = "stat")]
    Stats {
        #[command(subcommand)]
        command: SubCommandStats,
    },
}

#[derive(Debug, Subcommand)]
pub enum SubCommandJobs {
    /// Show job queue status
    #[command(visible_alias = "stat")]
    Status {
        /// Max count of job
        #[arg(long, short, default_value_t = 100)]
        max_count: usize,
    },

    /// Find tasks by friend code
    Find {
        /// Filter by friend code
        #[arg(long, short, value_name = "friend_code")]
        code: String,

        /// Max count of job
        #[arg(long, short, default_value_t = 100)]
        max_count: usize,
    },

    /// View task results
    #[command(visible_alias = "results")]
    Result {
        /// Filter by friend code
        #[arg(long, short, value_name = "friend_code")]
        code: String,

        /// Max count of job
        #[arg(long, short, default_value_t = 100)]
        max_count: usize,

        #[arg(long, short)]
        print_detail: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum SubCommandAccounts {
    /// Show account overview
    #[command(visible_alias = "stat")]
    Status {
        /// Max count of account
        #[arg(long, short, default_value_t = 100)]
        max_count: usize,
    },

    /// Account row operations
    Row {
        #[command(subcommand)]
        command: SubCommandAccountsRow,
    },

    /// Account rating operations
    Rate {
        #[command(subcommand)]
        command: SubCommandAccountsRate,

        /// Account ID
        #[arg(long, short)]
        id: i64,
    },

    /// Manually release an account (use with caution)
    Release {
        /// Account ID
        #[arg(long, short)]
        id: i64,

        /// Force release even with active lease (dangerous)
        #[arg(long)]
        force: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum SubCommandAccountsRow {
    /// Add an account
    Add {
        /// Username/email
        #[arg(long, short)]
        email: Option<String>,

        /// Password
        #[arg(long, short)]
        password: Option<String>,

        /// Read accounts from stdin (format: email:password per line)
        #[arg(long, default_value_t = false)]
        stdin: bool,
    },

    /// Remove an account
    #[command(visible_alias = "delete")]
    Remove {
        /// Account ID
        #[arg(long, short)]
        id: i64,
    },

    /// Query account details
    Query {
        /// Account ID
        #[arg(long, short)]
        id: i64,
    },
}

#[derive(Debug, Subcommand)]
pub enum SubCommandAccountsRate {
    /// Set or adjust rating
    Set {
        /// New rating value (or delta amount)
        #[arg(long, short)]
        value: i64,

        /// Treat value as a delta instead of absolute
        #[arg(long, short = 'd', default_value_t = true)]
        delta: bool,
    },

    /// Query current rating
    Query,
}

#[derive(Debug, Subcommand)]
pub enum SubCommandStats {
    /// Query worker state
    Worker {},

    /// Query worker event
    Event {
        #[arg(short, long, value_enum, default_value_t = ShowLevel::Info)]
        show_level: ShowLevel,

        #[arg(long, short)]
        limit: usize,
    },
}

#[derive(
    Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Default, Serialize, Deserialize,
)]
pub enum ShowLevel {
    Trace,
    #[default]
    Info,
    Warn,
    Fatal,
}

impl From<ShowLevel> for WorkerEventType {
    fn from(value: ShowLevel) -> Self {
        match value {
            ShowLevel::Trace => Self::Trace,
            ShowLevel::Info => Self::Info,
            ShowLevel::Warn => Self::Warn,
            ShowLevel::Fatal => Self::Fatal,
        }
    }
}
