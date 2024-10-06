use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder};
use sqlx::PgPool;
use tera::Tera;

use crate::auth::{login, signup, verify_token};
use crate::cron::{create_cron_job, get_user_cron_jobs};
use crate::models::{LoginUser, NewCronJob, NewUser};

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
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[post("/login")]
async fn login_route(pool: web::Data<PgPool>, form: web::Form<LoginUser>) -> impl Responder {
    match login(&pool, form.into_inner()).await {
        Ok(token) => HttpResponse::Found()
            .cookie(
                actix_web::cookie::Cookie::build("token", token)
                    .path("/")
                    .http_only(true)
                    .finish(),
            )
            .append_header(("Location", "/dashboard"))
            .finish(),
        Err(_) => HttpResponse::Unauthorized().finish(),
    }
}

#[post("/cron")]
async fn create_cron_job_route(
    pool: web::Data<PgPool>,
    form: web::Form<NewCronJob>,
    req: HttpRequest,
) -> impl Responder {
    let token = req
        .cookie("token")
        .and_then(|c| Some(c.value().to_string()));
    match token.and_then(|t| verify_token(&t).ok()) {
        Some(user_id) => match create_cron_job(&pool, user_id, form.into_inner()).await {
            Ok(_) => HttpResponse::Found()
                .append_header(("Location", "/dashboard"))
                .finish(),
            Err(_) => HttpResponse::InternalServerError().finish(),
        },
        None => HttpResponse::Unauthorized().finish(),
    }
}

#[get("/logout")]
async fn logout() -> impl Responder {
    HttpResponse::Found()
        .cookie(
            actix_web::cookie::Cookie::build("token", "")
                .path("/")
                .http_only(true)
                .max_age(actix_web::cookie::time::Duration::ZERO)
                .finish(),
        )
        .append_header(("Location", "/"))
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
