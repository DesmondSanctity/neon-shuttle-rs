use bcrypt::{hash, verify, DEFAULT_COST};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::models::{LoginUser, NewUser, User};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: i32,
    exp: usize,
}

pub async fn signup(pool: &PgPool, new_user: NewUser) -> Result<User, sqlx::Error> {
    let hashed_password = hash(new_user.password, DEFAULT_COST).unwrap();

    let user = sqlx::query!(
        r#"
        INSERT INTO users (username, email, password)
        VALUES ($1, $2, $3)
        RETURNING id, username, email, password, created_at
        "#,
        new_user.username,
        new_user.email,
        hashed_password
    )
    .fetch_one(pool)
    .await?;

    Ok(User {
        id: user.id,
        username: user.username,
        email: user.email,
        password: user.password,
        created_at: user.created_at.expect("Created at should be present"),
    })
}
pub async fn login(pool: &PgPool, login_user: LoginUser) -> Result<String, sqlx::Error> {
    let user = sqlx::query!(
        r#"
        SELECT id, username, email, password, created_at
        FROM users
        WHERE username = $1
        "#,
        login_user.username
    )
    .fetch_one(pool)
    .await?;

    if verify(login_user.password, &user.password).unwrap() {
        let claims = Claims {
            sub: user.id,
            exp: (chrono::Utc::now() + chrono::Duration::hours(24)).timestamp() as usize,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret("secret".as_ref()),
        )
        .unwrap();

        Ok(token)
    } else {
        Err(sqlx::Error::RowNotFound)
    }
}

pub fn verify_token(token: &str) -> Result<i32, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret("secret".as_ref()),
        &Validation::default(),
    )?;

    Ok(token_data.claims.sub)
}

pub async fn get_user(pool: &PgPool, user_id: i32) -> Result<User, sqlx::Error> {
    let user = sqlx::query!(
        r#"
        SELECT id, username, email, password, created_at
        FROM users
        WHERE id = $1
        "#,
        user_id
    )
    .fetch_one(pool)
    .await?;

    Ok(User {
        id: user.id,
        username: user.username,
        email: user.email,
        password: user.password,
        created_at: user.created_at.expect("Created at should not be null"),
    })
}
