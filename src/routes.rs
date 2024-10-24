use actix_web::cookie::Cookie;
use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder};
use serde_json::json;
use sqlx::PgPool;
use tera::Tera;
use tokio_cron_scheduler::JobScheduler;

use crate::auth::{login, signup, verify_token};
use crate::cron::{create_cron_job, get_user_cron_jobs, schedule_job};
use crate::models::{CronJob, LoginUser, NewCronJob, NewUser};

#[get("/")]
async fn index(tera: web::Data<Tera>) -> impl Responder {
    let context = tera::Context::new();
    let body = tera.render("index.html", &context).unwrap();
    HttpResponse::Ok().content_type("text/html").body(body)
}

#[get("/login")]
async fn login_page(tera: web::Data<Tera>) -> impl Responder {
    let context = tera::Context::new();
    let body = tera.render("login.html", &context).unwrap();
    HttpResponse::Ok().content_type("text/html").body(body)
}

#[get("/signup")]
async fn signup_page(tera: web::Data<Tera>) -> impl Responder {
    let context = tera::Context::new();
    let body = tera.render("signup.html", &context).unwrap();
    HttpResponse::Ok().content_type("text/html").body(body)
}

#[get("/dashboard")]
async fn dashboard(
    pool: web::Data<PgPool>,
    tera: web::Data<Tera>,
    req: HttpRequest,
) -> impl Responder {
    let token = req
        .cookie("token")
        .and_then(|c| Some(c.value().to_string()));
    match token.and_then(|t| verify_token(&t).ok()) {
        Some(user_id) => {
            let cron_jobs = get_user_cron_jobs(&pool, user_id).await.unwrap_or_default();
            let mut context = tera::Context::new();
            context.insert("cron_jobs", &cron_jobs);
            let body = tera.render("dashboard.html", &context).unwrap();
            HttpResponse::Ok().content_type("text/html").body(body)
        }
        None => HttpResponse::Found()
            .append_header(("Location", "/login"))
            .finish(),
    }
}

#[post("/signup")]
async fn signup_route(pool: web::Data<PgPool>, form: web::Form<NewUser>) -> impl Responder {
    match signup(&pool, form.into_inner()).await {
        Ok(_) => HttpResponse::Found()
            .append_header(("Location", "/login"))
            .finish(),
        Err(e) => HttpResponse::BadRequest()
            .content_type("application/json")
            .body(json!({ "error": format!("Signup failed: {}", e) }).to_string()),
    }
}

#[post("/login")]
async fn login_route(pool: web::Data<PgPool>, form: web::Form<LoginUser>) -> impl Responder {
    match login(&pool, form.into_inner()).await {
        Ok(json_value) => {
            if let Some(token) = json_value["token"].as_str() {
                HttpResponse::Found()
                    .cookie(
                        Cookie::build("token", token.to_string())
                            .path("/")
                            .http_only(true)
                            .finish(),
                    )
                    .append_header(("Location", "/dashboard"))
                    .finish()
            } else {
                HttpResponse::InternalServerError().json(json!({ "error": "Invalid token data" }))
            }
        }
        Err(e) => HttpResponse::BadRequest().json(e),
    }
}

#[post("/cron")]
async fn create_cron_job_route(
    pool: web::Data<PgPool>,
    scheduler: web::Data<JobScheduler>,
    form: web::Form<NewCronJob>,
    req: HttpRequest,
) -> impl Responder {
    let token = req
        .cookie("token")
        .and_then(|c| Some(c.value().to_string()));
    match token.and_then(|t| verify_token(&t).ok()) {
        Some(user_id) => match create_cron_job(&pool, user_id, form.into_inner()).await {
            Ok(json_value) => {
                if let Some(job) = json_value.get("job") {
                    let cron_job = CronJob {
                        id: job["id"].as_i64().unwrap() as i32,
                        user_id: job["user_id"].as_i64().unwrap() as i32,
                        message: job["message"].as_str().unwrap().to_string(),
                        schedule: job["schedule"].as_str().unwrap().to_string(),
                        last_run: None, // Adjust this based on your actual structure
                        created_at: chrono::DateTime::parse_from_rfc3339(
                            job["created_at"].as_str().unwrap(),
                        )
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                    };
                    if let Err(e) = schedule_job(scheduler, cron_job).await {
                        HttpResponse::InternalServerError()
                            .json(json!({ "error": format!("Failed to schedule job: {}", e) }))
                    } else {
                        HttpResponse::Found()
                            .append_header(("Location", "/dashboard"))
                            .finish()
                    }
                } else {
                    HttpResponse::InternalServerError().json(json!({ "error": "Invalid job data" }))
                }
            }
            Err(e) => HttpResponse::BadRequest().json(e),
        },
        None => HttpResponse::Unauthorized().json(json!({ "error": "Unauthorized" })),
    }
}

#[get("/logout")]
async fn logout() -> impl Responder {
    HttpResponse::Found()
        .append_header(("Location", "/login"))
        .cookie(
            actix_web::cookie::Cookie::build("token", "")
                .path("/")
                .http_only(true)
                .max_age(actix_web::cookie::time::Duration::ZERO)
                .finish(),
        )
        .finish()
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(index)
        .service(login_page)
        .service(signup_page)
        .service(dashboard)
        .service(signup_route)
        .service(login_route)
        .service(create_cron_job_route)
        .service(logout);
}
