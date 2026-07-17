use std::io::{self, BufRead};

use comfy_table::{ContentArrangement, Table};
use cuscuta_common::db::account::AccountRow;
use sqlx::postgres::PgPoolOptions;

/// Print an overview of all accounts: total count, state breakdown, accounts
/// with rate at or below zero (will never be picked up by workers), and
/// expired leases.
#[allow(clippy::cast_possible_truncation)]
pub async fn status(pg_url: &str, max_count: usize) -> anyhow::Result<()> {
    #[derive(sqlx::FromRow)]
    struct RowStateCount {
        state: String,
        cnt: i64,
    }
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(pg_url)
        .await?;
    let rows: Vec<RowStateCount> = sqlx::query_as(
        r"
        SELECT state, 
        COUNT(*) AS cnt 
        FROM account_table 
        GROUP BY state 
        ORDER BY state
        ",
    )
    .fetch_all(&pool)
    .await?;
    let (total,): (i64,) = sqlx::query_as(
        r"
        SELECT COUNT(*) 
        FROM account_table
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
    if max_count > 0 {
        let data: Vec<AccountRow> = sqlx::query_as(
            r"
        SELECT *
        FROM account_table 
        LIMIT $1
        ",
        )
        .bind(max_count.cast_signed() as i64)
        .fetch_all(&pool)
        .await?;
        let mut table = Table::new();
        table
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec!["id", "account_email", "status", "rate"]);
        for row in &data {
            table.add_row(vec![
                row.id.to_string(),
                row.account_email.clone(),
                row.state.clone(),
                row.rate.to_string(),
            ]);
        }
        println!("{table}");
        if total.cast_unsigned() as usize > max_count {
            println!(
                "... and {} more",
                total.cast_unsigned() as usize - max_count
            );
        }
    }
    pool.close().await;
    println!("Total accounts: {total}");
    for row in &rows {
        println!("  {:8}: {}", row.state, row.cnt);
    }
    if rate_zero > 0 {
        println!("  rate<=0 : {rate_zero} (will not be assigned)");
    }
    if expired > 0 {
        println!("  expired leases: {expired}");
    }
    Ok(())
}

/// Add one or more accounts.
///
/// In normal mode, `--email` and `--password` are required.  With `--stdin`,
/// lines are read from standard input in `email:password` format (blank lines
/// and lines starting with `#` are skipped).
pub async fn row_add(
    pg_url: &str,
    email: Option<String>,
    password: Option<String>,
    stdin: bool,
) -> anyhow::Result<()> {
    let entries: Vec<(String, String)> = if stdin {
        let mut list = Vec::new();
        let reader = io::stdin().lock();
        for line in reader.lines() {
            let line = line?;
            let line = line.trim().to_string();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            match line.split_once(':') {
                Some((e, p)) => list.push((e.to_string(), p.to_string())),
                None => {
                    eprintln!("warning: skipping malformed line (expected email:password): {line}");
                }
            }
        }
        if list.is_empty() {
            anyhow::bail!("no valid entries read from stdin");
        }
        list
    } else {
        let email = email.ok_or_else(|| anyhow::anyhow!("--email is required"))?;
        let password = password.ok_or_else(|| anyhow::anyhow!("--password is required"))?;
        vec![(email, password)]
    };
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(pg_url)
        .await?;
    let mut added = 0;
    for (email, password) in &entries {
        let result = sqlx::query(
            r"
            INSERT 
            INTO account_table 
            (account_email, account_password) 
            VALUES ($1, $2)",
        )
        .bind(email)
        .bind(password)
        .execute(&pool)
        .await;
        match result {
            Ok(_) => {
                println!("added: {email}");
                added += 1;
            }
            Err(e) => {
                eprintln!("failed to add {email}: {e}");
            }
        }
    }
    pool.close().await;
    println!(
        "\n{added}/{total} added successfully.",
        total = entries.len()
    );
    Ok(())
}

/// Delete an account by its database id.
pub async fn row_remove(pg_url: &str, id: i64) -> anyhow::Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(pg_url)
        .await?;

    let result = sqlx::query(
        r"
        DELETE 
        FROM account_table 
        WHERE id = $1
        ",
    )
    .bind(id)
    .execute(&pool)
    .await?;

    pool.close().await;

    if result.rows_affected() == 0 {
        println!("No account found with id={id}");
    } else {
        println!("Removed account id={id}");
    }
    Ok(())
}

/// Print the full row for a single account (password is truncated).
pub async fn row_query(pg_url: &str, id: i64) -> anyhow::Result<()> {
    #[derive(sqlx::FromRow)]
    struct AccountRow {
        id: i64,
        account_email: String,
        account_password: String,
        user_id: Option<i64>,
        temp_token: Option<String>,
        state: String,
        rate: i32,
        lease_time: chrono::DateTime<chrono::Utc>,
    }
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(pg_url)
        .await?;
    let row: Option<AccountRow> = sqlx::query_as(
        "SELECT id, account_email, account_password, user_id, temp_token, state, rate, lease_time FROM account_table WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&pool)
    .await?;
    pool.close().await;
    match row {
        Some(r) => {
            println!("id:       {}", r.id);
            println!("email:    {}", r.account_email);
            println!(
                "password: {}...",
                &r.account_password.chars().take(4).collect::<String>()
            );
            println!(
                "user_id:  {}",
                r.user_id.map_or_else(|| "-".to_string(), |v| v.to_string())
            );
            println!(
                "token:    {}",
                r.temp_token
                    .as_deref()
                    .map_or("-", |t| { if t.len() > 12 { &t[..12] } else { t } })
            );
            println!("state:    {}", r.state);
            println!("rate:     {}", r.rate);
            println!("lease:    {}", r.lease_time);
        }
        None => println!("No account found with id={id}"),
    }
    Ok(())
}

/// Set or adjust the rating (`rate`) of an account.
///
/// By default `--value` is treated as a delta (`rate = rate + value`).
/// Pass `--delta=false` to set an absolute value instead.
#[allow(clippy::cast_possible_truncation)]
pub async fn rate_set(pg_url: &str, id: i64, value: i64, delta: bool) -> anyhow::Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(pg_url)
        .await?;

    let new_rate: Option<i32> = if delta {
        sqlx::query_scalar("UPDATE account_table SET rate = rate + $2 WHERE id = $1 RETURNING rate")
            .bind(id)
            .bind(value as i32)
            .fetch_optional(&pool)
            .await?
    } else {
        sqlx::query_scalar("UPDATE account_table SET rate = $2 WHERE id = $1 RETURNING rate")
            .bind(id)
            .bind(value as i32)
            .fetch_optional(&pool)
            .await?
    };
    pool.close().await;
    match new_rate {
        Some(r) => println!("Account id={id} rate updated to {r}"),
        None => println!("No account found with id={id}"),
    }
    Ok(())
}

/// Query the current rating of an account.
pub async fn rate_query(pg_url: &str, id: i64) -> anyhow::Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(pg_url)
        .await?;

    let rate: Option<i32> = sqlx::query_scalar(
        r"
        SELECT rate 
        FROM account_table 
        WHERE id = $1
        ",
    )
    .bind(id)
    .fetch_optional(&pool)
    .await?;

    pool.close().await;

    match rate {
        Some(r) => println!("Account id={id} rate = {r}"),
        None => println!("No account found with id={id}"),
    }
    Ok(())
}

/// Release (set to `Idle`) an account that is currently `Using`.
///
/// Refuses to release an account whose lease has not yet expired unless
/// `--force` is passed.
pub async fn release(pg_url: &str, id: i64, force: bool) -> anyhow::Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(pg_url)
        .await?;

    let (state, lease_time): (String, chrono::DateTime<chrono::Utc>) = sqlx::query_as(
        r"
            SELECT state, lease_time 
            FROM account_table 
            WHERE id = $1
            ",
    )
    .bind(id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| anyhow::anyhow!("No account found with id={id}"))?;

    if state == "Idle" {
        println!("Account id={id} is already Idle.");
        pool.close().await;
        return Ok(());
    }

    let now = chrono::Utc::now();
    if !force && lease_time > now {
        anyhow::bail!(
            "Account id={id} has an active lease (expires at {lease_time}). \
             Use --force to override."
        );
    }

    sqlx::query(
        r"
        UPDATE account_table 
        SET state = 'Idle' 
        WHERE id = $1",
    )
    .bind(id)
    .execute(&pool)
    .await?;

    pool.close().await;
    println!("Account id={id} released (was {state}).");
    Ok(())
}
