# Data extraction

Three styles, listed in order of preference for typical REST APIs.

## 1. Typed extractors as parameters (preferred)

```rust
use salvo::prelude::*;
use salvo::extract::{JsonBody, PathParam, QueryParam}; // salvo_core since 0.96 — NOT in prelude, no `oapi` needed
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)] // Serialize because `create` echoes it back as Json<NewUser>
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

Since **0.96** these types live in **`salvo_core::extract`** — import them as `salvo::extract::{...}`; they work **without** the `oapi` feature (`CookieParam` needs the default-on `cookie` feature). The full set: `JsonBody`, `FormBody`, `FormFile`, `FormFiles`, `PathParam`, `QueryParam`, `HeaderParam`, `CookieParam`. They are still **not** in `salvo::prelude`. With `oapi` on, `salvo::oapi::extract::{...}` re-exports the very same types (so pre-0.96 imports keep compiling) and they double as schema sources for `#[endpoint]` — either path works; prefer `salvo::extract` in new code.

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

There is **no `parse_msgpack`** (still absent in 1.1.1) — for MessagePack, read `req.payload().await` and decode with `rmp-serde` yourself.

**`.await` gotcha:** only the body parsers (`parse_json` / `parse_form` / `parse_body`) are `async` — `.await` them. The non-body family (`parse_queries` / `parse_params` / `parse_headers` / `parse_cookies`) is **synchronous** — call without `.await`: `let q: MyQuery = req.parse_queries()?;`. Over-awaiting them is `E0277` (Result is not a Future).

## Validation with `validator`

Salvo doesn't ship a validator — pair `JsonBody<T>` with the `validator` crate:

```toml
validator = { version = "0.18", features = ["derive"] }
```

```rust
use validator::Validate;

#[derive(Deserialize, Serialize, Validate)] // Serialize: the handler returns Json<NewUser>
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

`req.file("name")` returns `Option<&FilePart>`. For multiple files under one field name use `req.files("name")`, which returns `Option<&Vec<FilePart>>`. `req.try_file("name")` returns `ParseResult<Option<&FilePart>>` when you need to tell "no such field" apart from "multipart parse failed".

Or take them as **typed extractors** — the parameter name is the form field name:

```rust
use salvo::extract::{FormFile, FormFiles};

#[handler]
async fn upload(avatar: FormFile, attachments: FormFiles) -> String {
    format!("{:?} + {} attachments", avatar.name(), attachments.len())
}
```

`FilePart` / `FormFile` expose:
- `.path()` — the temp file on disk
- `.name()` — the original filename
- `.content_type()` — the MIME type
- `.headers()` — all part headers
- `.size()` — byte length

Temp-file lifetime (1.1+): the file stays on disk until the **last** `FilePart` clone / owned `FormFile` / `FormFiles` is dropped — an owned `FormFile` keeps it readable even after the request ends (handy for handing it to a background task). Borrowed `&FilePart` from `req.file(...)` dies with the request, so copy/move the file before returning if you only hold the borrow.

**Multipart count limits (1.1+):** parsing is capped at **128 parts and 32 files** by default; exceeding either (or the byte limit) returns **413**, not 400. Raise or lower per route with the `FormLimits` middleware — `Router::with_path("upload").hoop(FormLimits::new().max_files(200)).post(upload)` — or `req.set_form_limits(...)` inside a handler. Byte limits (`size-limiter`, the 64KB default body cap) apply independently — see `files.md`.
