use cuscuta_common::db::{
    job::track::fetch_all_job_track_tag,
    redis::{job_result_tracking_redis_key, job_result_value_redis_key},
};
use redis::{ScanOptions, TypedCommands};

/// List all active job streams with their message counts, pending messages,
/// and consumer counts.
pub fn status(redis_url: &str, max_count: usize) -> anyhow::Result<()> {
    let client = redis::Client::open(redis_url)?;
    let mut con = client.get_connection()?;

    let keys: Vec<String> = con
        .scan_options::<String>(
            ScanOptions::default()
                .with_count(max_count)
                .with_pattern("cuscuta:jobs:*"),
        )?
        .collect::<Result<Vec<_>, _>>()?;

    if keys.is_empty() {
        println!("No job streams found.");
        return Ok(());
    }

    println!(
        "{:48} {:>8} {:>8} {:>8}",
        "STREAM", "MSGS", "PENDING", "CONSUMERS"
    );
    println!("{:-<80}", "");

    for key in &keys {
        let len: usize = con.xlen(key).unwrap_or(0);
        let pending: usize = con
            .xpending(key, "default_group")
            .map_or(0, |it| it.count());
        let xinfo_result = con
            .xinfo_consumers(key, "default_group")
            .unwrap_or_default();
        let short_name = if key.len() > 46 {
            format!("{}...", &key[..43])
        } else {
            key.clone()
        };
        println!(
            "{short_name:48} {len:>8} {pending:>8} {cons:>8}",
            cons = if xinfo_result.consumers.is_empty() {
                "-".to_string()
            } else {
                xinfo_result.consumers.len().to_string()
            }
        );
    }
    println!("\n{total} streams total.", total = keys.len());
    Ok(())
}

/// Search result indices for a given friend code and display each matching
/// [`JobTag`] entry.
pub fn find(redis_url: &str, friend_code: &str, max_count: usize) -> anyhow::Result<()> {
    let client = redis::Client::open(redis_url)?;
    let mut con = client.get_connection()?;

    let pattern = job_result_tracking_redis_key(&format!("{friend_code}-*"));
    let keys: Vec<String> = con
        .scan_options(
            ScanOptions::default()
                .with_count(max_count)
                .with_pattern(pattern),
        )?
        .collect::<Result<Vec<_>, _>>()?;

    if keys.is_empty() {
        println!("No results found for friend_code={friend_code}");
        return Ok(());
    }

    for key in &keys {
        println!("\n--- {key} ---");
        let entries = fetch_all_job_track_tag(&client, key)?;
        for tag in &entries {
            println!("  status:   {:?}", tag.status);
            println!("  uid:      {}", tag.job_essential.job_uid);
            println!("  queue:    {}", tag.queue.name);
            println!("  segment:  {:?}", tag.queue.segment);
            println!(
                "  cursor:   {}..{}",
                tag.job_essential.cursor_start, tag.job_essential.cursor_length
            );
            println!("  retries:  {}", tag.job_essential.retry_count);
            println!("  ids:      {}", tag.job_ids.join(","));
            println!("  failures: {:?}", tag.failures);
        }
    }

    Ok(())
}

/// Fetch and display score results for a given friend code.
///
/// When `print_detail` is `false`, only the total count is printed;
/// otherwise each [`SongScore`](cuscuta_common::api::xxxxxx::SongScore)
/// entry is displayed with its key fields.
pub fn result(
    redis_url: &str,
    friend_code: &str,
    max_count: usize,
    print_detail: bool,
) -> anyhow::Result<()> {
    let client = redis::Client::open(redis_url)?;
    let mut con = client.get_connection()?;

    let pattern = job_result_value_redis_key(&format!("{friend_code}-*"));
    let keys: Vec<String> = con
        .scan_options(
            ScanOptions::default()
                .with_count(max_count)
                .with_pattern(pattern),
        )?
        .collect::<Result<Vec<_>, _>>()?;

    if keys.is_empty() {
        println!("No result data found for friend_code={friend_code}");
        return Ok(());
    }

    let mut total = 0usize;
    for key in &keys {
        let entries: Vec<String> = con.lrange(key, 0, -1)?;
        for entry in &entries {
            total += 1;
            if !print_detail {
                continue;
            }
            match serde_json::from_str::<cuscuta_common::api::xxxxxx::SongScore>(entry) {
                Ok(score) => {
                    println!();
                    println!("  song_id:     {}", score.song_id);
                    println!("  player:      {} ({})", score.player_name, score.user_id);
                    println!("  difficulty:  {}", score.difficulty);
                    println!(
                        "  score:       {} (below max: {})",
                        score.score, score.score_below_max
                    );
                    println!(
                        "  shiny/perfect/near/miss:  {}/{}/{}/{}",
                        score.shiny_perfect_count,
                        score.perfect_count,
                        score.near_count,
                        score.miss_count
                    );
                    println!(
                        "  clear_type:  {} (best: {})",
                        score.clear_type, score.best_clear_type
                    );
                }
                Err(_) => {
                    println!("  (unparseable) {entry}");
                }
            }
        }
    }
    println!("\n{total} result(s) total.");
    Ok(())
}
