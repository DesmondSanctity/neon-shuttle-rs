use actix_web::{web, App, HttpServer};
use dotenv::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::env;
use tera::Tera;
use tokio_cron_scheduler::JobScheduler;

mod auth;
mod cron;
mod models;
mod routes;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(std::time::Duration::from_secs(30))
        .connect(&database_url)
        .await
        .unwrap_or_else(|e| {
            log::error!("Failed to connect to the database: {}", e);
            panic!("Database connection error. Check your connection settings and try again.");
        });

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .unwrap_or_else(|e| {
            log::error!("Failed to run database migrations: {}", e);
            panic!("Database migration error. Check your connection settings and try again.");
        });

    let tera = Tera::new("templates/**/*").expect("Failed to initialize Tera");

    // Initialize the job scheduler
    let scheduler = JobScheduler::new()
        .await
        .expect("Failed to create scheduler");
    let scheduler_data = web::Data::new(scheduler.clone());

    // Start the scheduler
    scheduler.start().await.expect("Failed to start scheduler");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(tera.clone()))
            .app_data(scheduler_data.clone())
            .configure(routes::config)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
