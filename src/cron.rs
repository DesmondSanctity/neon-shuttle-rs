use actix_web::web;
use serde_json::json;
use sqlx::PgPool;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::models::{CronJob, NewCronJob};

pub async fn create_cron_job(
    pool: &PgPool,
    user_id: i32,
    new_cron_job: NewCronJob,
) -> Result<serde_json::Value, serde_json::Value> {
    match sqlx::query!(
        r#"
        INSERT INTO cron_jobs (user_id, message, schedule)
        VALUES ($1, $2, $3)
        RETURNING id, user_id, message, schedule, last_run, created_at
        "#,
        user_id,
        new_cron_job.message,
        new_cron_job.schedule
    )
    .fetch_one(pool)
    .await
    {
        Ok(job) => Ok(json!({
            "message": "Cron job created successfully",
            "job": {
                "id": job.id,
                "user_id": job.user_id,
                "message": job.message,
                "schedule": job.schedule,
                "last_run": job.last_run,
                "created_at": job.created_at
            }
        })),
        Err(e) => Err(json!({ "error": format!("Failed to create cron job: {}", e) })),
    }
}

pub async fn get_user_cron_jobs(
    pool: &PgPool,
    user_id: i32,
) -> Result<serde_json::Value, serde_json::Value> {
    match sqlx::query!(
        r#"
        SELECT id, user_id, message, schedule, last_run, created_at
        FROM cron_jobs
        WHERE user_id = $1
        ORDER BY created_at DESC
        "#,
        user_id
    )
    .fetch_all(pool)
    .await
    {
        Ok(jobs) => Ok(json!({
            "cron_jobs": jobs.into_iter().map(|job| json!({
                "id": job.id,
                "user_id": job.user_id,
                "message": job.message,
                "schedule": job.schedule,
                "last_run": job.last_run,
                "created_at": job.created_at
            })).collect::<Vec<_>>()
        })),
        Err(e) => Err(json!({ "error": format!("Failed to fetch cron jobs: {}", e) })),
    }
}

pub async fn schedule_job(
    scheduler: web::Data<JobScheduler>,
    cron_job: CronJob,
) -> Result<(), Box<dyn std::error::Error>> {
    let message = cron_job.message.clone();
    let job = Job::new_async(cron_job.schedule.as_str(), move |_, _| {
        let message = message.clone();
        Box::pin(async move {
            println!("Executing cron job: {}", message);
        })
    })?;
    scheduler.add(job).await?;
    Ok(())
}
