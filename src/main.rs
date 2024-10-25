use std::sync::Arc;
use axum::{
    routing::{get, post},
    http::StatusCode,
    Json, Router,
};
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use rand::{Rng};
use sqlx::{ConnectOptions, Pool, Postgres, Row};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tracing::log;

#[derive(Clone)]
pub struct AppState {
    db_pool: Pool<Postgres>,
}

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    println!("🌟 REST API Service 🌟");

    let options = PgConnectOptions::new()
        .host("localhost")
        .username("postgres")
        .password("pwd")
        .database("postgres")
        .port(54320)
        .log_statements(log::LevelFilter::Trace);

    // 1) Create a connection pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(options) //.connect("postgres://postgres:pwd@localhost:54320/postgres")
        .await?;

    let state = AppState {
        db_pool: pool.clone()
    };

    // "host=localhost user=postgres password=pwd port=54320"

    println!("✅ Connection to the database is successful!");

    sqlx::query("
        CREATE TABLE IF NOT EXISTS users (
            id              SERIAL PRIMARY KEY,
            username        VARCHAR NOT NULL,
            email           VARCHAR NOT NULL
            )
    ")
        .execute(&state.db_pool)
        .await?;

    println!("✅ Client table created!");

    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/", get(hello))
        .route("/users", get(get_users))
        .route("/users/search", post(search_user))
        .route("/users", post(create_user))
        .with_state(Arc::from(state));

    let listener =
        tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, app).await.unwrap();

    Ok(())
}

#[axum::debug_handler]
async fn hello() -> (StatusCode, String) {
    //Ok(Json("Hello from Rust first REST API with axum :)".to_string()))
    (StatusCode::OK, "Hello from Rust first REST API with axum :)".to_string())
}

async fn get_users(state: State<Arc<AppState>>) -> Result<Json<Vec<User>>, AppError> {

    let users: Vec<User> = sqlx::query("select * from users")
        .fetch_all(&state.db_pool)
        .await?
        .iter_mut()
        .map(|row| User {
            id: row.get(0),
            username: row.get(1),
            email_address: row.get(2),
        }).collect();
    //.iter()
    //.collect();


    Ok(Json(users))
}

#[derive(Deserialize)]
enum Ordering {
    ASC,
    DESC
}

#[derive(Deserialize)]
struct UserSearchCriteria {
    criteria: String,
    ordered: Option<Ordering>
}

#[axum::debug_handler]
async fn search_user(state: State<Arc<AppState>>, Json(criteria): Json<UserSearchCriteria>)
    -> Result<Json<Vec<User>>, AppError> {

    let mut ordering_to_bind = "".to_string();
    if criteria.ordered.is_some() {
        ordering_to_bind = match criteria.ordered.unwrap() {
            Ordering::ASC => "order by username ASC".to_string(),
            Ordering::DESC => "order by username DESC".to_string()
        };
    }

    let criteria_clone = criteria.criteria.clone();

    let search_user_query = format!(
        "select * from users
        where username like '%{}%'
        or email like '%{}%' {}",
        criteria.criteria,
        criteria_clone,
        ordering_to_bind
    );

    let users: Vec<User> = sqlx::query(&search_user_query)
        .fetch_all(&state.db_pool)
        .await?
        .iter_mut()
        .map(|row| User {
            id: row.get(0),
            username: row.get(1),
            email_address: row.get(2)
        })
        .collect();

    /*let users: Vec<User> = sqlx::query("
                select * from users
                 where username like '%' || $1 || '%'
                 or email like '%' || $2 || '%'
                 $3 ")
        .bind(criteria.criteria)
        .bind(criteria_clone)
        .bind(" order by username".to_string())
        .fetch_all(&state.db_pool)
        .await?
        .iter_mut()
        .map(|row| User {
            id: row.get(0),
            username: row.get(1),
            email_address: row.get(2)
        })
        .collect();*/


    Ok(Json(users))
}

#[axum::debug_handler]
async fn create_user(state: State<Arc<AppState>>, Json(payload): Json<CreateUserQuery>)
                     -> Result<Json<CreateUserResponse>, AppError> {
    let rng = rand::thread_rng().gen::<u32>();
    let id = rng.count_ones();
    let key = payload.email_address.clone();
    let clone_user_name = payload.username.clone();

    println!("id={}, name={}, email={}", id, clone_user_name, key);

    let result = sqlx::query("\
        insert into users(username, email) \
        values($1, $2);")
        //.bind(id.to_string())
        .bind(&clone_user_name)
        .bind(key)
        .execute(&state.db_pool)
        .await?;

    println!("result={:?}", result);

    let create_user_response = CreateUserResponse {
        id: id.clone(),
        username: payload.username.clone(),
    };

    Ok(Json(create_user_response))
}


#[derive(Deserialize)]
struct CreateUserQuery {
    username: String,
    email_address: String,
}

#[derive(Serialize)]
struct User {
    id: i32,
    username: String,
    email_address: String,
}

#[derive(Serialize)]
struct CreateUserResponse {
    id: u32,
    username: String,
}

struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
