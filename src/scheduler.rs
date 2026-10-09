use std::sync::Arc;
use anyhow::Result;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info};
use crate::{orchestrator, state::AppState};

pub async fn start(state: Arc<AppState>) -> Result<JobScheduler> {
    let scheduler = JobScheduler::new().await?;
    scheduler.add(Job::new_async("0 */5 * * * *", move |_id, _lock| {
        let state = Arc::clone(&state);
        Box::pin(async move {
            info!("Scheduled SKU scan started");
            match orchestrator::process_all(state).await {
                Ok(results) => info!(count = results.len(), "Scheduled SKU scan completed"),
                Err(error) => error!(%error, "Scheduled SKU scan failed"),
            }
        })
    })?).await?;
    scheduler.start().await?;
    Ok(scheduler)
}
