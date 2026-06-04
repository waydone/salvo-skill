# Error handling

## The two-axis design

1. **Per-handler errors**: a handler returns `Result<T, E>` where both arms implement `Writer`. The error variant becomes the response.
2. **Catcher**: a service-level filter that runs when a response has a 4xx/5xx status code AND empty body — this is where you customize 404 / 500 pages globally.

You will use both. Per-handler errors handle business-specific failures; the catcher provides a uniform 404 / unhandled-error page.

## Per-handler: `StatusError` (recommended for most APIs)

```rust
use salvo::prelude::*;
use salvo::http::StatusError;

#[handler]
async fn get_user(req: &mut Request) -> Result<Json<User>, StatusError> {
    let id: i64 = req.param("id").ok_or_else(StatusError::bad_request)?;

    let user = db::find(id).await
        .map_err(|e|
            StatusError::internal_server_error()
                .brief("database error")
                .detail(e.to_string())
        )?;

    user.map(Json).ok_or_else(StatusError::not_found)
}
```

`StatusError` constructors cover the common codes: `bad_request`, `unauthorized`, `forbidden`, `not_found`, `method_not_allowed`, `conflict`, `unprocessable_entity`, `too_many_requests`, `internal_server_error`, `service_unavailable`, etc. Plus `.brief(...)` and `.detail(...)` for human-readable messages.

`StatusError`'s `Writer` impl content-negotiates: HTML by default, JSON if `Accept: application/json`, etc.

## Per-handler: custom error type (when you need a uniform JSON envelope)

When the API contract is e.g. `{ "code": <int>, "msg": "...", "data": ... }`:

```rust
use async_trait::async_trait;
use salvo::prelude::*;

pub enum AppError {
    BadInput(String),
    NotFound,
    Internal(anyhow::Error),
}

impl<E: Into<anyhow::Error>> From<E> for AppError {
    fn from(e: E) -> Self { AppError::Internal(e.into()) }
}

#[async_trait]
impl Writer for AppError {
    async fn write(self, _req: &mut Request, _depot: &mut Depot, res: &mut Response) {
        let (code, status, msg) = match self {
            AppError::BadInput(m) => (1001, StatusCode::BAD_REQUEST, m),
            AppError::NotFound    => (1002, StatusCode::NOT_FOUND, "not found".into()),
            AppError::Internal(e) => (5000, StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };
        res.status_code(status);
        res.render(Json(serde_json::json!({ "code": code, "msg": msg, "data": null })));
    }
}

#[handler]
async fn show(req: &mut Request) -> Result<Json<User>, AppError> {
    let id: i64 = req.param("id").ok_or_else(|| AppError::BadInput("missing id".into()))?;
    let u = db::find(id).await?.ok_or(AppError::NotFound)?;
    Ok(Json(u))
}
```

## `anyhow` (with the `anyhow` Cargo feature)

```toml
salvo = { version = "0.93.0", features = ["full", "anyhow"] }
```

```rust
#[handler]
async fn h() -> Result<&'static str, anyhow::Error> {
    let _config = std::fs::read_to_string("/missing")?;  // → 500 with body = error string
    Ok("ok")
}
```

`anyhow::Error` maps to 500. Same applies to `eyre::Report` with the `eyre` feature. Keep production-grade APIs to `StatusError` or a custom type — `anyhow::Error` leaks internal errors to clients.

## Catcher — global 404 / fallback

```rust
use salvo::catcher::Catcher;
use salvo::prelude::*;

#[handler]
async fn handle404(res: &mut Response, ctrl: &mut FlowCtrl) {
    if matches!(res.status_code, Some(StatusCode::NOT_FOUND)) {
        res.render(Json(serde_json::json!({"error": "not found"})));
        ctrl.skip_rest();
    }
}

#[handler]
async fn handle5xx(res: &mut Response, ctrl: &mut FlowCtrl) {
    if let Some(code) = res.status_code {
        if code.is_server_error() && res.body.is_none() {
            res.render(Json(serde_json::json!({"error": "internal", "code": code.as_u16()})));
            ctrl.skip_rest();
        }
    }
}

let service = Service::new(router)
    .catcher(Catcher::default().hoop(handle404).hoop(handle5xx));
```

The catcher only runs when the response body is empty. Handlers that already wrote a body (e.g. `Json(my_error)`) skip it.

`Catcher::default()` already provides a sensible HTML/JSON/XML/plain default for empty 4xx/5xx — you only need to override when you want a specific format.

## Panic recovery

For panics inside handlers, enable `catch-panic`:

```toml
features = [..., "catch-panic"]
```

```rust
use salvo::catch_panic::CatchPanic;

let app = Service::new(Router::new().hoop(CatchPanic::new()).get(handler));
```

A panic becomes a 500. **Do not** rely on this in place of proper error handling — it exists for true bugs, not normal failure cases.

## Logging errors

Pair errors with `tracing`. A common pattern is to log at the conversion boundary:

```rust
.map_err(|e| {
    tracing::error!(?e, "db error in get_user");
    StatusError::internal_server_error()
})
```

Avoid leaking the original error to the client (`StatusError::internal_server_error()` without `.detail(...)` keeps the response generic). The detail goes in your logs, not the response.
