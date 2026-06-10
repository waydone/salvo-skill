# Data extraction

Three styles, listed in order of preference for typical REST APIs.

## 1. Typed extractors as parameters (preferred)

```rust
use salvo::prelude::*;
use salvo::oapi::extract::{JsonBody, PathParam, QueryParam}; // extractors are NOT in prelude (oapi feature)
use serde::Deserialize;

#[derive(Deserialize)]
struct NewUser { name: String, email: String }

#[handler]
async fn create(json: JsonBody<NewUser>) -> Json<NewUser> {
    Json(json.into_inner())
}

#[handler]
async fn show(id: PathParam<i64>) -> String {
    format!("id={}", id.into_inner())
}

#[handler]
async fn search(q: QueryParam<String, true>, page: QueryParam<u32, false>) -> String {
    format!("q={} page={:?}", q.into_inner(), page.into_inner())
}
```

The `bool` const generic on `QueryParam` / `HeaderParam` / `CookieParam` is `REQUIRED`:
- `QueryParam<T, true>` — fail with 400 if missing
- `QueryParam<T, false>` — yields `Option<T>`-ish via `.into_inner()`

These types are in `salvo::oapi::extract` and need the `oapi` feature. They double as schema sources for OpenAPI generation. Without `oapi`, use `Request` methods or `Extractible` derive instead; `salvo::extract` does not provide these `JsonBody` / `PathParam` / `QueryParam` wrapper types in 0.93.0.

## 2. `Extractible` derive (mixed sources, complex shapes)

```rust
use salvo::macros::Extractible;
use serde::Deserialize;

#[derive(Deserialize, Extractible, Debug)]
#[salvo(extract(default_source(from = "body")))]
struct UpdateUser<'a> {
    #[salvo(extract(source(from = "param")))]
    id: i64,

    #[salvo(extract(source(from = "query")))]
    notify: bool,

    name: &'a str,           // body (default)
    email: &'a str,          // body (default)
}

#[handler]
async fn update(req: &mut Request, depot: &mut Depot) -> Result<(), StatusError> {
    let u: UpdateUser<'_> = req.extract(depot).await
        .map_err(|_| StatusError::bad_request())?;
    /* ... */
    Ok(())
}
```

Source values:
- `"param"` — path parameter
- `"query"` — URL query string
- `"header"` — request header
- `"cookie"` — request cookie
- `"body"` — request body (parsed as JSON or form depending on `Content-Type`)
- `"request"` — the whole `Request` object (rare)

Borrowed types (`&str`, `&[u8]`) work — they avoid an allocation. The lifetime threads through `Request`.

## 3. Imperative (use when extraction is conditional)

```rust
#[handler]
async fn h(req: &mut Request) -> Result<String, StatusError> {
    // Path parameter
    let id: i64 = req.param("id").ok_or_else(StatusError::bad_request)?;

    // Query parameter
    let q: Option<String> = req.query("q");

    // Header
    let auth: Option<String> = req.header::<String>("authorization");

    // Cookie (cookie feature)
    let session = req.cookie("session_id").map(|c| c.value().to_string());

    // JSON body
    let body: NewUser = req.parse_json::<NewUser>().await
        .map_err(|_| StatusError::bad_request())?;

    // Form body
    let form: MyForm = req.parse_form().await
        .map_err(|_| StatusError::bad_request())?;

    // Either JSON or form depending on Content-Type
    let mixed: MyData = req.parse_body().await
        .map_err(|_| StatusError::bad_request())?;

    Ok("ok".to_string())
}
```

`req.parse_*` family:
- `parse_json::<T>()` — JSON body (async)
- `parse_form::<T>()` — `application/x-www-form-urlencoded` or `multipart/form-data` (async)
- `parse_body::<T>()` — auto-detect JSON vs form from `Content-Type` (async)
- `parse_queries::<T>()` — URL query string (sync; note: plural `parse_queries`, not `parse_query`)
- `parse_params::<T>()` — path params (sync)
- `parse_headers::<T>()` — headers (sync)
- `parse_cookies::<T>()` — cookies (sync, cookie feature)

There is **no `parse_msgpack`** in 0.93.0 — for MessagePack, read `req.payload().await` and decode with `rmp-serde` yourself.

**`.await` gotcha:** only the body parsers (`parse_json` / `parse_form` / `parse_body`) are `async` — `.await` them. The non-body family (`parse_queries` / `parse_params` / `parse_headers` / `parse_cookies`) is **synchronous** — call without `.await`: `let q: MyQuery = req.parse_queries()?;`. Over-awaiting them is `E0277` (Result is not a Future).

## Validation with `validator`

Salvo doesn't ship a validator — pair `JsonBody<T>` with the `validator` crate:

```toml
validator = { version = "0.18", features = ["derive"] }
```

```rust
use validator::Validate;

#[derive(Deserialize, Validate)]
struct NewUser {
    #[validate(email)]
    email: String,
    #[validate(length(min = 3, max = 30))]
    name: String,
}

#[handler]
async fn create(json: JsonBody<NewUser>) -> Result<Json<NewUser>, StatusError> {
    let u = json.into_inner();
    u.validate().map_err(|e|
        StatusError::bad_request().brief("invalid input").detail(e.to_string())
    )?;
    Ok(Json(u))
}
```

Or factor it into a `Validated<T>` wrapper that implements `Extractible` — useful at app scale.

## File uploads (multipart)

```rust
#[handler]
async fn upload(req: &mut Request) -> Result<String, StatusError> {
    let file = req.file("avatar").await
        .ok_or_else(StatusError::bad_request)?;
    let dest = std::path::PathBuf::from("/tmp").join(
        file.name().unwrap_or("upload.bin")
    );
    std::fs::copy(file.path(), &dest)
        .map_err(|e| StatusError::internal_server_error().detail(e.to_string()))?;
    Ok(format!("saved to {}", dest.display()))
}
```

`req.file("name")` returns `Option<&FilePart>`. For multiple files under one field name use `req.files("name")`, which returns `Option<&Vec<FilePart>>`.

`FilePart` exposes:
- `.path()` — the temp file on disk
- `.name()` — the original filename
- `.content_type()` — the MIME type
- `.headers()` — all part headers
- `.size()` — byte length

The temp file is cleaned up when `Request` drops, so move/copy it before the handler returns.
