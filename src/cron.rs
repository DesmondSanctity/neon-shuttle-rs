use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::models::{CronJob, NewCronJob};

pub async fn create_cron_job(
    pool: &PgPool,
    user_id: i32,
    new_cron_job: NewCronJob,
) -> Result<CronJob, sqlx::Error> {
    let cron_job = sqlx::query!(
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
    .await?;

    Ok(CronJob {
        id: cron_job.id,
        user_id: cron_job.user_id.expect("User ID should be present"),
        message: cron_job.message,
        schedule: cron_job.schedule,
        last_run: cron_job.last_run,
        created_at: cron_job.created_at.expect("Created at should be present"),
    })
}
pub async fn get_user_cron_jobs(pool: &PgPool, user_id: i32) -> Result<Vec<CronJob>, sqlx::Error> {
    let cron_jobs = sqlx::query!(
        r#"
        SELECT id, user_id, message, schedule, last_run, created_at
        FROM cron_jobs
        WHERE user_id = $1
        ORDER BY created_at DESC
        "#,
        user_id
    )
    .fetch_all(pool)
    .await?;

    Ok(cron_jobs
        .into_iter()
        .map(|job| CronJob {
            id: job.id,
            user_id: job.user_id.expect("User ID should be present"),
            message: job.message,
            schedule: job.schedule,
            last_run: job.last_run,
            created_at: job.created_at.expect("Created at should be present"),
        })
        .collect())
}

pub async fn run_cron_jobs(pool: &PgPool) -> Result<(), sqlx::Error> {
    let now: DateTime<Utc> = Utc::now();

    let cron_jobs = sqlx::query!(
        r#"
        SELECT id, user_id, message, schedule, last_run, created_at
        FROM cron_jobs
        WHERE last_run IS NULL OR last_run < $1
        "#,
        now
    )
    .fetch_all(pool)
    .await?;

    for job in cron_jobs {
        println!("Running cron job: {}", job.message);

        sqlx::query!(
            r#"
            UPDATE cron_jobs
            SET last_run = $1
            WHERE id = $2
            "#,
            now,
            job.id
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}

