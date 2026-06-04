# Database integration

Salvo doesn't bundle an ORM. Pick one and inject the connection pool via `affix_state`. SQLx is the most common choice; SeaORM and Diesel work too.

## SQLx (recommended default)

```toml
[dependencies]
salvo = { version = "0.93.0", features = ["oapi", "logging", "affix-state"] }
sqlx = { version = "0.8", features = ["runtime-tokio", "tls-rustls", "postgres", "macros", "chrono", "uuid"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
serde = { version = "1", features = ["derive"] }
```

```rust
use salvo::prelude::*;
use salvo::oapi::extract::JsonBody;
use sqlx::{PgPool, postgres::PgPoolOptions};
use serde::{Deserialize, Serialize};

#[derive(sqlx::FromRow, Serialize, Deserialize, ToSchema)]
struct Article {
    id: i64,
    title: String,
    body: String,
}

#[derive(Deserialize, ToSchema)]
struct NewArticle { title: String, body: String }

#[endpoint(tags("articles"))]
async fn list_articles(depot: &mut Depot) -> Result<Json<Vec<Article>>, StatusError> {
    let pool = depot.obtain::<PgPool>()
        .map_err(|_| StatusError::internal_server_error())?;
    let rows: Vec<Article> = sqlx::query_as("SELECT id, title, body FROM articles ORDER BY id DESC")
        .fetch_all(pool)
        .await
        .map_err(|e| {
            tracing::error!(?e, "list_articles");
            StatusError::internal_server_error()
        })?;
    Ok(Json(rows))
}

#[endpoint(tags("articles"), status_codes(201))]
async fn create_article(
    depot: &mut Depot,
    body: JsonBody<NewArticle>,
) -> Result<Json<Article>, StatusError> {
    let pool = depot.obtain::<PgPool>()
        .map_err(|_| StatusError::internal_server_error())?;
    let NewArticle { title, body } = body.into_inner();
    let row: Article = sqlx::query_as(
        "INSERT INTO articles (title, body) VALUES ($1, $2) RETURNING id, title, body"
    )
    .bind(&title).bind(&body)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        tracing::error!(?e, "create_article");
        StatusError::internal_server_error()
    })?;
    Ok(Json(row))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&std::env::var("DATABASE_URL")?)
        .await?;

    let router = Router::new()
        .hoop(salvo::affix_state::inject(pool))
        .push(Router::with_path("articles").get(list_articles).post(create_article));

    let acceptor = TcpListener::new("0.0.0.0:5800").bind().await;
    Server::new(acceptor).serve(router).await;
    Ok(())
}
```

Notes:
- `depot.obtain::<PgPool>()` returns `&PgPool`. SQLx accepts `&PgPool` directly for queries — no clone needed.
- For multi-database apps (multiple pools), inject newtypes: `inject(ReadPool(read_pool)).inject(WritePool(write_pool))`, then `depot.obtain::<ReadPool>()`.
- Run migrations at startup with `sqlx::migrate!("./migrations").run(&pool).await?` before binding the listener.

## SeaORM

```toml
sea-orm = { version = "1", features = ["sqlx-postgres", "runtime-tokio-rustls", "macros"] }
```

```rust
use sea_orm::{Database, DatabaseConnection};

#[handler]
async fn handler(depot: &mut Depot) -> Result<Json<Vec<MyModel::Model>>, StatusError> {
    let db = depot.obtain::<DatabaseConnection>()
        .map_err(|_| StatusError::internal_server_error())?;
    let rows = MyModel::Entity::find().all(db).await
        .map_err(|_| StatusError::internal_server_error())?;
    Ok(Json(rows))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db = Database::connect(std::env::var("DATABASE_URL")?).await?;
    let app = Router::new().hoop(salvo::affix_state::inject(db)).push(/* routes */);
    /* ... */ Ok(())
}
```

## Diesel (sync) — wrap in `tokio::task::spawn_blocking`

Diesel's sync API doesn't fit cleanly in async handlers. Either:
1. Use `diesel-async` (recommended) — works the same way as SQLx above.
2. Wrap each query in `tokio::task::spawn_blocking`. This is fine for low-traffic admin tooling but adds latency.

## Repository pattern

Once you have more than ~5 endpoints, factor queries into a repository struct:

```rust
#[derive(Clone)]
pub struct ArticleRepo { pool: PgPool }

impl ArticleRepo {
    pub async fn list(&self) -> sqlx::Result<Vec<Article>> {
        sqlx::query_as("...").fetch_all(&self.pool).await
    }
    pub async fn create(&self, new: &NewArticle) -> sqlx::Result<Article> {
        sqlx::query_as("...").bind(...).fetch_one(&self.pool).await
    }
}
```

Inject `ArticleRepo` (cheap to clone — it just holds the pool) and call methods from handlers. Tests can swap the repo for an in-memory mock.

## Transactions

```rust
let mut tx = pool.begin().await?;
sqlx::query("INSERT ...").execute(&mut *tx).await?;
sqlx::query("UPDATE ...").execute(&mut *tx).await?;
tx.commit().await?;
```

In a Salvo handler this looks the same — just remember to map errors. Don't hold transactions across `.await` points that don't need them; keep them short to free connections.

## Migration

For sqlx, prefer `sqlx::migrate!()` at startup:

```rust
sqlx::migrate!("./migrations").run(&pool).await?;
```

Migrations live in `./migrations/` as `<timestamp>_<name>.sql`. Generate with `sqlx-cli`.
