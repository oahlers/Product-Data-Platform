use crate::{orchestrator, state::AppState};
use anyhow::Result;
use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info};

pub async fn start(state: Arc<AppState>) -> Result<JobScheduler> {
    let scheduler = JobScheduler::new().await?;

    let cron = std::env::var("SCHEDULER_CRON").unwrap_or_else(|_| "0 */5 * * * *".to_string());

    info!(cron = %cron, "Starting scheduler");

    scheduler
        .add(Job::new_async(&cron, move |_id, _lock| {
            let state = Arc::clone(&state);

            Box::pin(async move {
                info!("Scheduled incremental Inriver scan started");

                match orchestrator::process_modified(state).await {
                    Ok(results) => {
                        info!(
                            count = results.len(),
                            "Scheduled incremental Inriver scan completed"
                        );
                    }
                    Err(error) => {
                        error!(
                            %error,
                            "Scheduled incremental Inriver scan failed"
                        );
                    }
                }
            })
        })?)
        .await?;

    scheduler.start().await?;

    Ok(scheduler)
}
