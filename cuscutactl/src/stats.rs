use chrono::{TimeZone, Utc};
use cuscuta_common::db::log::{WorkerEventType, event::read_events, status::search_worker_status};

use crate::command::ShowLevel;

/// Fetch worker status
pub fn worker(redis_url: &str) -> anyhow::Result<()> {
    let client = redis::Client::open(redis_url)?;
    let result = search_worker_status(&client)?;
    for (k, stat) in result {
        println!("- Worker: {k}");
        println!("  active_timestamp: {}", stat.last_active_timestamp);
        println!("  cursor:           {}", stat.cursor);
        println!(
            "  sub_queue:        {}",
            stat.sub_queue
                .as_ref()
                .map_or("None", |sub_queue| &sub_queue.name)
        );
        println!("  jobs:");
        for it in stat.jobs.iter() {
            println!("    essential: {:?}", it.essential);
            println!("    stat:      {:?}", it.state);
            println!("    id:        {:?}", it.job_id);
        }
        println!();
    }
    Ok(())
}

/// Fetch worker events
pub fn event(redis_url: &str, show_level: ShowLevel, limit: usize) -> anyhow::Result<()> {
    let client = redis::Client::open(redis_url)?;
    let mut out = Vec::new();
    let mut lines = 0;
    loop {
        let news = read_events(&client, lines, limit)?;
        let len = news.len();
        if len == 0 {
            break;
        }
        out.append(
            &mut news
                .into_iter()
                .filter(|it| it.event_type as u8 >= WorkerEventType::from(show_level) as u8)
                .collect::<Vec<_>>(),
        );
        lines += len;
        if lines >= limit {
            break;
        }
    }
    for it in out {
        let opt = Utc.timestamp_opt(it.timestamp / 1000, 0);
        println!(
            "{:?} {:?} {:?} {}",
            opt, it.event_type, it.worker_id, it.message
        );
    }
    Ok(())
}
