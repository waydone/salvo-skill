# Testing (feature `test`)

Salvo ships a `TestClient` that drives a `Service` in-process — no real socket involved. It's fast and gives you full control over headers, bodies, and assertions.

```toml
[dev-dependencies]
salvo = { version = "0.93.0", features = ["test"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
serde_json = "1"
```

## Setup

Factor route construction so tests can reuse it:

```rust
// src/lib.rs (or src/routes.rs)
use salvo::prelude::*;

#[handler]
pub async fn hello() -> &'static str { "Hello" }

pub fn router() -> Router {
    Router::new().get(hello)
}
```

```rust
// src/main.rs
#[tokio::main]
async fn main() {
    let acceptor = TcpListener::new("0.0.0.0:5800").bind().await;
    Server::new(acceptor).serve(myapp::router()).await;
}
```

## A first test

```rust
#[cfg(test)]
mod tests {
    use salvo::prelude::*;
    use salvo::test::{ResponseExt, TestClient};

    #[tokio::test]
    async fn root_returns_hello() {
        let service = Service::new(super::router());

        let mut res = TestClient::get("http://localhost/")
            .send(&service)
            .await;

        assert_eq!(res.status_code, Some(StatusCode::OK));
        let body = res.take_string().await.unwrap();
        assert_eq!(body, "Hello");
    }
}
```

Notes:
- The host in the URL is irrelevant — `TestClient` doesn't open sockets. `http://localhost/...` is conventional.
- `TestClient::get / post / put / patch / delete / options` mirrors HTTP methods.
- `.send(&service).await` returns a `Response`. Use `.take_string()` / `.take_json::<T>()` / `.take_bytes()` to consume the body.

## Sending JSON

```rust
#[tokio::test]
async fn create_user() {
    let service = Service::new(super::router());

    let body = serde_json::json!({ "name": "Ada", "email": "ada@example.com" });

    let mut res = TestClient::post("http://localhost/users")
        .json(&body)
        .send(&service)
        .await;

    assert_eq!(res.status_code, Some(StatusCode::CREATED));
    let user: User = res.take_json().await.unwrap();
    assert_eq!(user.name, "Ada");
}
```

## Sending headers / query / form

```rust
TestClient::get("http://localhost/api/me")
    .add_header("authorization", format!("Bearer {token}"), true)
    .query("expand", "profile")
    .send(&service).await;

TestClient::post("http://localhost/login")
    .form(&[("user", "ada"), ("pass", "secret")])
    .send(&service).await;
```

## Multipart upload

```rust
TestClient::post("http://localhost/upload")
    .file("avatar", "./tests/fixtures/avatar.png")
    .send(&service).await;
```

## Testing middleware (e.g. JWT)

```rust
#[tokio::test]
async fn protected_route_rejects_anonymous() {
    let service = Service::new(super::router());

    let res = TestClient::get("http://localhost/api/me").send(&service).await;
    assert_eq!(res.status_code, Some(StatusCode::UNAUTHORIZED));
}

#[tokio::test]
async fn protected_route_accepts_valid_token() {
    let service = Service::new(super::router());

    let token = mint_test_token();   // your helper that signs with the test secret

    let mut res = TestClient::get("http://localhost/api/me")
        .add_header("authorization", format!("Bearer {token}"), true)
        .send(&service).await;

    assert_eq!(res.status_code, Some(StatusCode::OK));
}
```

## Database tests — pattern: per-test transaction

```rust
async fn make_service(pool: PgPool) -> Service {
    let router = Router::new()
        .hoop(salvo::affix_state::inject(pool))
        .push(myapp::routes());
    Service::new(router)
}

#[sqlx::test(migrations = "./migrations")]
async fn list_articles_returns_seeded(pool: PgPool) {
    sqlx::query("INSERT INTO articles (title, body) VALUES ('a', 'b')")
        .execute(&pool).await.unwrap();

    let service = make_service(pool).await;
    let mut res = TestClient::get("http://localhost/articles").send(&service).await;
    assert_eq!(res.status_code, Some(StatusCode::OK));

    let articles: Vec<Article> = res.take_json().await.unwrap();
    assert_eq!(articles.len(), 1);
}
```

`#[sqlx::test]` gives you a fresh transaction per test, rolled back at the end. Combined with `affix_state`, every test gets isolated state.

## Common assertions

```rust
assert_eq!(res.status_code, Some(StatusCode::OK));
assert!(res.headers().get("content-type").unwrap().to_str().unwrap().starts_with("application/json"));
let body = res.take_string().await.unwrap();
assert!(body.contains("expected substring"));
```

For structured assertions on JSON, prefer `take_json::<MyDto>()` and assert on fields. Falling back to `serde_json::Value` is fine for ad-hoc tests.

## Tips

- **Don't `unwrap` in production code** but it's fine in tests — failures should panic loudly.
- **Use `tokio::time::pause()`** if you need to advance time deterministically (e.g. testing rate limiters).
- **For WebSocket / SSE tests**, `TestClient` doesn't speak those protocols. Run the real server on a random port and connect a real client (`tungstenite`, `eventsource-client`).
- **Don't share a single `Service` across tests that mutate global state**. Build one per test or use isolation (transactions, per-test temp dirs).
