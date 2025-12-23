use sqlx::PgPool;
use tokio_cron_scheduler::{JobScheduler, Job};
use std::time::Duration;

pub async fn start_scheduler(pool: PgPool) -> anyhow::Result<()> {
    let mut sched = JobScheduler::new().await?;

    // Daily leaderboard update at 00:05 UTC
    let pool_clone = pool.clone();
    sched.add(Job::new_async("5 0 * * *", move |_uuid, _l| {
        let pool = pool_clone.clone();
        Box::pin(async move {
            if let Err(e) = crate::engine::leaderboard::update_all_leaderboards(&pool).await {
                log::error!("Leaderboard update failed: {}", e);
            } else {
                log::info!("Daily leaderboard update completed successfully");
            }
        })
    })?).await?;

    // // Optional: Weekly deep sync from Allium (every Sunday at 02:00 UTC)
    // let pool_clone = pool.clone();
    // sched.add(Job::new_async("0 2 * * SUN", move |_uuid, _l| {
    //     let pool = pool_clone.clone();
    //     Box::pin(async move {
    //         if let Err(e) = crate::engine::historical_sync::full_sync_from_allium(&pool).await {
    //             log::error!("Weekly Allium sync failed: {}", e);
    //         } else {
    //             log::info!("Weekly Allium historical sync completed");
    //         }
    //     })
    // })?).await?;

    sched.start().await?;
    Ok(())
}
