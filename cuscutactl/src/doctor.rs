use std::time::Instant;

use redis::{ScanOptions, TypedCommands};
use sqlx::postgres::PgPoolOptions;

/// Run a series of diagnostic checks against the cuscuta databases.
///
/// Checks performed (when the corresponding URL is provided):
///
/// - `PostgreSQL` connectivity (`SELECT version()`)
/// - Migration status (presence of `account_table`)
/// - Account overview (state breakdown, expired leases)
/// - Redis connectivity (`PING`)
/// - Active job streams and orphan detection
///
/// Both `pg_url` and `redis_url` are optional; checks that lack a URL are
/// skipped with a `[ -- ]` indicator.
pub async fn run(pg_url: Option<&String>, redis_url: Option<&String>) -> anyhow::Result<()> {
    let mut all_ok = true;
    let all_missing = pg_url.is_none() && redis_url.is_none();

    if let Some(pg_url) = pg_url {
        match check_pg(pg_url).await {
            Ok(version) => println!("  PostgreSQL  [ ok ] connected ({version})"),
            Err(e) => {
                println!("  PostgreSQL  [FAIL] {e}");
                all_ok = false;
            }
        }

        match check_migration(pg_url).await {
            Ok(()) => println!("  Migration   [ ok ] account_table exists"),
            Err(e) => {
                println!("  Migration   [FAIL] {e}");
                all_ok = false;
            }
        }

        match check_accounts(pg_url).await {
            Ok(summary) => println!("  Accounts    [ ok ] {summary}"),
            Err(e) => {
                println!("  Accounts    [FAIL] {e}");
                all_ok = false;
            }
        }
    } else {
        println!("  PostgreSQL  [ -- ] skipped (no PostgreSQL URL configured)");
        println!("  Migration   [ -- ] skipped (no PostgreSQL URL configured)");
        println!("  Accounts    [ -- ] skipped (no PostgreSQL URL configured)");
    }

    if let Some(redis_url) = redis_url {
        match check_redis(redis_url) {
            Ok(()) => println!("  Redis       [ ok ] PONG"),
            Err(e) => {
                println!("  Redis       [FAIL] {e}");
                all_ok = false;
            }
        }

        match check_streams(redis_url) {
            Ok(summary) => println!("  Job Streams [ ok ] {summary}"),
            Err(e) => {
                println!("  Job Streams [FAIL] {e}");
                all_ok = false;
            }
        }
    } else {
        println!("  Redis       [ -- ] skipped (no Redis URL configured)");
        println!("  Job Streams [ -- ] skipped (no Redis URL configured)");
    }

    println!();
    if all_missing {
        println!("Nothing was checked.");
    } else if all_ok {
        println!("All checks passed.");
    } else {
        println!("Some checks failed. Review the items above.");
    }
    Ok(())
}

async fn check_pg(pg_url: &str) -> anyhow::Result<String> {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(pg_url)
        .await?;
    let (version,): (String,) = sqlx::query_as("SELECT version()").fetch_one(&pool).await?;
    pool.close().await;
    let short = version.split(',').next().unwrap_or(&version).to_string();
    Ok(short)
}

async fn check_migration(pg_url: &str) -> anyhow::Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(pg_url)
        .await?;
    let exists: bool = sqlx::query_scalar(
        r"
        SELECT EXISTS
        (SELECT FROM information_schema.tables WHERE table_name = 'account_table')
        ",
    )
    .fetch_one(&pool)
    .await?;
    pool.close().await;
    if exists {
        Ok(())
    } else {
        anyhow::bail!("account_table not found; run migrations first")
    }
}

async fn check_accounts(pg_url: &str) -> anyhow::Result<String> {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(pg_url)
        .await?;
    let (total,): (i64,) = sqlx::query_as(
        r"
        SELECT COUNT(*) FROM account_table
        ",
    )
    .fetch_one(&pool)
    .await?;
    let (idle,): (i64,) = sqlx::query_as(
        r"
        SELECT COUNT(*) 
        FROM account_table 
        WHERE state = 'Idle'
        ",
    )
    .fetch_one(&pool)
    .await?;
    let (using,): (i64,) = sqlx::query_as(
        r"
        SELECT COUNT(*) 
        FROM account_table 
        WHERE state = 'Using'
        ",
    )
    .fetch_one(&pool)
    .await?;
    let (rate_zero,): (i64,) = sqlx::query_as(
        r"
    SELECT COUNT(*) 
    FROM account_table 
    WHERE rate <= 0
    ",
    )
    .fetch_one(&pool)
    .await?;
    let (expired,): (i64,) = sqlx::query_as(
        r"
        SELECT COUNT(*) 
        FROM account_table 
        WHERE state = 'Using' AND lease_time < now()
        ",
    )
    .fetch_one(&pool)
    .await?;
    pool.close().await;

    let mut parts = vec![format!("{total} total")];
    if idle > 0 {
        parts.push(format!("{idle} Idle"));
    }
    if using > 0 {
        parts.push(format!("{using} Using"));
    }
    if rate_zero > 0 {
        parts.push(format!("{rate_zero} rate<=0"));
    }
    if expired > 0 {
        parts.push(format!("{expired} expired lease"));
    }
    Ok(parts.join(" | "))
}

fn check_redis(redis_url: &str) -> anyhow::Result<()> {
    let client = redis::Client::open(redis_url)?;
    let mut con = client.get_connection()?;
    con.ping()?;
    Ok(())
}

fn check_streams(redis_url: &str) -> anyhow::Result<String> {
    let client = redis::Client::open(redis_url)?;
    let mut con = client.get_connection()?;
    let start = Instant::now();

    let keys: Vec<String> = con
        .scan_options::<String>(
            ScanOptions::default()
                .with_count(100)
                .with_pattern("cuscuta:jobs:*"),
        )?
        .collect::<Result<Vec<_>, _>>()?;

    let total = keys.len();
    if total == 0 {
        return Ok("0 streams".to_string());
    }

    let mut orphans = 0;
    for key in &keys {
        if con
            .xinfo_groups(key)
            .unwrap_or_default()
            .groups
            .iter()
            .map(|it| it.consumers)
            .sum::<usize>()
            == 0
        {
            orphans += 1;
        }
    }

    let elapsed = start.elapsed();
    let mut parts = vec![format!("{total} streams")];
    if orphans > 0 {
        parts.push(format!("{orphans} orphans"));
    }
    parts.push(format!("({elapsed:.1?})"));
    Ok(parts.join(" | "))
}
