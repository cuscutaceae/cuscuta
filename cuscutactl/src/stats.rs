use cuscuta_common::db::log::status::search_worker_status;

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
