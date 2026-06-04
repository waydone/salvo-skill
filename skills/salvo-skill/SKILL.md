---
name: salvo-skill
description: "Salvo Rust web framework specialist (target 0.93.0). USE whenever ANY apply. (1) ANY mention of 'salvo'/'Salvo', even casual ('Salvo 项目报 404', 'use salvo::', 'salvo = 0.92'). (2) Cargo.toml has salvo = '…', or source imports salvo/salvo_core/salvo_extra/salvo_oapi. (3) Salvo symbols — #[handler], #[endpoint], Router::with_path, Depot, JwtAuth, WebSocketUpgrade, SwaggerUi, affix_state::inject, StatusError. (4) Fresh Rust web/API project (REST, WebSocket, SSE, reverse proxy, OpenAPI, JWT, internal dashboard, LLM streaming, real-time push) where user does NOT pick axum/actix/rocket/warp/poem — Salvo is this user's default. (5) Salvo debugging — route 404, missing JwtAuth, Cargo feature missing, Message::Text fails, angle-bracket path, missing Writer impl, empty OpenAPI. (6) Migrating TO Salvo. SKIP axum/actix/rocket/warp/poem when staying inside, hyper/tower direct, non-Rust stacks, jsonwebtoken without web. Bundles 0.93.0 API + Context7 rule for non-trivial code."
---

# Salvo Skill (target: 0.93.0)

You are helping the user build Rust web services with **Salvo** — a modular, ergonomic Rust web framework. The user develops AI-first: they expect you to make sound default choices and produce production-grade code without round-tripping for clarification on every detail.

This skill encodes Salvo's 0.93.0 API surface and a workflow for keeping that surface accurate as the library evolves.

### 0.93.0 vs 0.92.2 — non-breaking

0.93.0 (2026-04-30) is a **non-breaking** release over 0.92.2. The core API this skill documents (handlers, routing, `JwtAuth`, `Writer`/`Scribe`, `Depot`, CORS, static serving, OpenAPI) is unchanged — code written for 0.92.2 compiles on 0.93.0. The only changes are four internal fixes, relevant only if you touch those areas:

- **Proxy**: stricter proxy path normalization (`proxy` feature).
- **CSRF**: enforces a minimum bcrypt CSRF token size.
- **Rate limiter**: atomic guard verification in `MokaStore`.
- **TUS uploads**: tightened tus disk-store path validation.

If a future release introduces breaking changes, re-verify with Context7 and bump this section. Latest stable as of this skill: **0.93.0**.

## Hard rule: verify with Context7 before non-trivial code

Salvo's API has changed meaningfully across recent minor versions (path syntax, OpenAPI macros, middleware crate splits). Your training data is older than 0.93.0.

**Before writing any non-trivial Salvo code, query Context7.** Non-trivial means anything beyond `fn main` + `Router::new().get(hello)`.

Use this exact pattern:

```
Tool: mcp__plugin_context7_context7__query-docs
libraryId: /websites/rs_salvo
query: <a specific question, e.g. "Salvo 0.93 JwtAuth middleware constructor and decoder configuration">
```

Fallback library IDs if the first returns nothing useful:
- `/salvo-rs/salvo` — GitHub source (smaller corpus, sometimes fresher)
- `/websites/salvo_rs_zh-hans` — Chinese-language docs (use if user writes in Chinese and a translated example would help)

Skip Context7 only when the user is just asking conceptual questions ("what is Salvo?") or when the snippet you need is already verbatim in `references/` and you're sure it matches 0.93.0.

If Context7 errors out, tell the user and fall back to your knowledge — but flag the API surface as unverified and recommend running `cargo check` before trusting it.

## Project skeleton — paste-ready

This is the minimum that compiles on 0.93.0. Use it as the starting point for any new app.

`Cargo.toml`:

```toml
[package]
name = "myapp"
version = "0.1.0"
edition = "2024"   # ⚠️ for NEW projects use 2024 (current, Rust 1.85+). Do NOT reflexively write "2021" — that's a stale training-data default. For an EXISTING project, match whatever its Cargo.toml already declares.

[dependencies]
salvo = { version = "0.93.0", features = ["full"] } # add "size-limiter" explicitly for upload/body caps
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
serde = { version = "1", features = ["derive"] }
tracing = "0.1"
tracing-subscriber = "0.3"
```

`features = ["full"]` is fine for prototyping, but it does not include `size-limiter`. For production, switch to a curated list — see `references/cargo-features.md`. Common minimal sets:

- REST API w/ OpenAPI: `["oapi", "logging", "affix-state"]`
- WebSocket service: `["websocket", "logging"]`
- Static + reverse proxy gateway: `["serve-static", "proxy", "compression", "logging"]`

`src/main.rs`:

```rust
use salvo::prelude::*;

#[handler]
async fn hello() -> &'static str {
    "Hello from Salvo"
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let router = Router::new().get(hello);
    let acceptor = TcpListener::new("0.0.0.0:5800").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

Notes:
- `salvo::prelude::*` re-exports everything you usually need: `Router`, `Server`, `TcpListener`, `Request`, `Response`, `Depot`, `FlowCtrl`, `Writer`, `StatusError`, `#[handler]`, plus feature-gated items like `JwtAuth`, `Compression`, `WebSocketUpgrade`, `Scalar`, `SwaggerUi`.
- The 8698 port shown in docs.rs examples is just a docs convention. Use 5800 (Salvo community default) or whatever fits the user's setup.

## Handler signatures

`#[handler]` inspects parameters **by type**, not by position. Take exactly the ones you need, in any order, and skip the rest. The macro generates the right adapter.

```rust
#[handler]
async fn h1() -> &'static str { "ok" }

#[handler]
async fn h2(req: &mut Request) -> String { /* ... */ }

#[handler]
async fn h3(depot: &mut Depot) -> Json<User> { /* ... */ }                    // ✅ depot only

#[handler]
async fn h4(res: &mut Response, depot: &mut Depot) { /* ... */ }              // ✅ any order

#[handler]
async fn h5(req: &mut Request, depot: &mut Depot,
            res: &mut Response, ctrl: &mut FlowCtrl) { /* ... */ }            // all four
```

The macro recognizes four "context" parameters by their exact types:

| Parameter type | What you use it for |
|---|---|
| `&mut Request` | Read incoming headers, body, URL, params |
| `&mut Depot` | Read/write request-scoped state (DI) |
| `&mut Response` | Write status, headers, body manually |
| `&mut FlowCtrl` | Skip rest of chain (`ctrl.skip_rest()`) |

In addition, the macro accepts **typed extractors** as parameters (`JsonBody<T>`, `PathParam<T>`, `QueryParam<T, REQUIRED>`, `HeaderParam<T, REQUIRED>`, `CookieParam<T, REQUIRED>`). These can appear alongside the context parameters above.

Return types: any `T: Writer`. The framework provides impls for `&str`, `String`, `Json<T>`, `Text<...>`, `Redirect`, `()` (no body), `Result<T, E>` where both arms are `Writer`, `StatusError`, and (with the `anyhow` feature) `anyhow::Error` → 500.

For a return type cheatsheet, see `references/handlers.md`.

## Routing — the syntax that bites people

Salvo 0.76+ uses **curly-brace** path parameters: `{id}`, `{**rest}`. The old `<id>` / `<**>` colon syntax is gone.

```rust
Router::with_path("articles/{id}").get(show);            // /articles/42
Router::with_path("files/{**rest}").get(serve);          // /files/a/b/c
Router::with_path(r"users/{id|\d+}").get(by_id);         // regex constraint
```

Common patterns:

```rust
let router = Router::new()
    .hoop(Logger::new())                  // global middleware
    .get(index)                           // GET /
    .push(
        Router::with_path("api/v1")       // /api/v1/...
            .hoop(jwt_auth)               // scoped middleware
            .push(Router::with_path("users").get(list_users).post(create_user))
            .push(Router::with_path("users/{id}").get(get_user).put(update_user).delete(delete_user))
    );
```

Method shortcuts on `Router`: `.get(h)`, `.post(h)`, `.put(h)`, `.patch(h)`, `.delete(h)`, `.head(h)`, `.options(h)`. Use `.goal(h)` when **only this handler** should match (e.g. WebSocket upgrade routes — see `references/realtime.md`).

For nested routers, custom filters, host/scheme matching, and the `and()` / `or()` composition rules, see `references/routing.md`.

## Data extraction — three styles

**1. Imperative (use when extraction is conditional):**

```rust
#[handler]
async fn h(req: &mut Request) -> Result<String, StatusError> {
    let id: i64 = req.param("id").ok_or_else(StatusError::bad_request)?;
    let q: String = req.query("q").unwrap_or_default();
    let body: MyForm = req.parse_form().await.map_err(|_| StatusError::bad_request())?;
    Ok(format!("..."))
}
```

**2. Typed extractors as parameters (idiomatic, terse):**

```rust
use salvo::oapi::extract::{JsonBody, PathParam, QueryParam}; // extractors are NOT in prelude

#[handler]
async fn create(json: JsonBody<NewUser>) -> Json<User> {
    let NewUser { name, email } = json.into_inner();
    /* ... */
}
```

Available: `JsonBody<T>`, `FormBody<T>`, `QueryParam<T, REQUIRED>`, `PathParam<T>`, `HeaderParam<T, REQUIRED>`, `CookieParam<T, REQUIRED>` — all in `salvo::oapi::extract` (requires `oapi`), **not** in `salvo::prelude`. Without `oapi`, use `Request` methods or `Extractible` derive instead.

**3. `Extractible` derive (for mixed-source structs):**

```rust
#[derive(Deserialize, Extractible, Debug)]
#[salvo(extract(default_source(from = "body")))]
struct UpdateUser<'a> {
    #[salvo(extract(source(from = "param")))]
    id: i64,
    #[salvo(extract(source(from = "query")))]
    notify: bool,
    name: &'a str,           // from body (default)
    email: &'a str,          // from body (default)
}
```

Sources: `"param"` (path), `"query"`, `"header"`, `"cookie"`, `"body"`, `"request"` (whole request).

For validation (combining with `validator` crate), file uploads, and the full extractible attribute reference, see `references/data-extraction.md`.

## Middleware (`hoop`) and shared state (`Depot` + `affix_state`)

Middleware is anything that implements `Handler`. Attach with `.hoop(...)`:

```rust
let router = Router::new()
    .hoop(Logger::new())                              // request-scoped
    .hoop(affix_state::inject(db_pool.clone()))       // inject typed state
    .get(hello);
```

Reading injected state in a handler:

```rust
#[handler]
async fn hello(depot: &mut Depot) -> Result<String, StatusError> {
    let pool = depot.obtain::<PgPool>().map_err(|_| StatusError::internal_server_error())?;
    /* ... */
}
```

`depot.obtain::<T>()` looks up by **type**. `depot.get::<&str>("key")` and `depot.insert("key", val)` use string keys — handy for per-request data.

For writing custom middleware, the onion model, `FlowCtrl::skip_rest()`, conditional middleware via `hoop_when`, and DI patterns, see `references/middleware-state.md`.

## Errors

Two patterns, pick one and stick with it:

**A. `StatusError` for HTTP-shaped errors** (recommended for REST APIs):

```rust
#[handler]
async fn get_user(req: &mut Request) -> Result<Json<User>, StatusError> {
    let id: i64 = req.param("id").ok_or_else(StatusError::bad_request)?;
    let user = db::find(id).await
        .map_err(|e| StatusError::internal_server_error().brief("db error").detail(e.to_string()))?;
    user.ok_or_else(StatusError::not_found).map(Json)
}
```

**B. Custom error type implementing `Writer`** (when you want per-app JSON error envelope):

```rust
struct AppError(anyhow::Error);

#[async_trait]
impl Writer for AppError {
    async fn write(self, _req: &mut Request, _depot: &mut Depot, res: &mut Response) {
        res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
        res.render(Json(serde_json::json!({ "error": self.0.to_string() })));
    }
}
```

Don't mix `anyhow::Result<T>` directly into handler returns unless the `anyhow` Cargo feature is on — without it there's no `Writer` impl for `anyhow::Error`.

For 404 / 500 catch-all pages, custom error envelopes, and the `Catcher` API, see `references/error-handling.md`.

## When to consult `references/`

Read these only when the task touches the topic. Each file is self-contained — you don't need to read them in order.

| File | Read it when… |
|---|---|
| `references/cargo-features.md` | Choosing Cargo feature flags, debugging a "function not found" error from a missing feature, or trimming binary size |
| `references/routing.md` | Custom filters, deep nesting, host/scheme matching, regex parameters, `goal` vs `get` |
| `references/handlers.md` | Return-type quirks, `Writer` impls, streaming responses |
| `references/data-extraction.md` | File uploads, multipart, validation, custom extractors |
| `references/middleware-state.md` | Writing custom middleware, `FlowCtrl`, DI patterns, scoped state |
| `references/error-handling.md` | Custom error envelopes, `Catcher`, 404 pages |
| `references/auth-security.md` | JWT, basic auth, sessions, CSRF, CORS, rate limiting, TLS/ACME |
| `references/openapi.md` | `#[endpoint]`, `ToSchema`, `ToParameters`, SwaggerUi / Scalar / RapiDoc / ReDoc setup |
| `references/realtime.md` | WebSocket, SSE, broadcast patterns |
| `references/database.md` | SQLx / SeaORM / Diesel integration via `affix_state` |
| `references/files.md` | Static directories, embedded assets (rust-embed), file uploads |
| `references/ops.md` | Logging/tracing, timeout, graceful shutdown, compression, proxy, HTTP/2/3 |
| `references/testing.md` | `TestClient`, integration tests, mocking middleware |

If the user's question doesn't fit a single file, read 1–2 candidates and synthesize. Don't read all of them speculatively.

## Anti-patterns to avoid

These come up repeatedly in AI-generated Salvo code. Catch them in your own output before submitting.

1. **Old path syntax**: `with_path("articles/<id>")` — wrong since 0.76. Use `{id}`.
2. **Forgetting Cargo features**: writing `JwtAuth::new(...)` without `features = ["jwt-auth"]` (or `"full"`) → "cannot find struct" compile error. When you reach for a non-core type, mention the feature flag inline.
3. **Returning raw `anyhow::Error`** without enabling the `anyhow` feature → no `Writer` impl, won't compile.
4. **Mixing `axum`/`actix` idioms**: Salvo handlers don't use tuple extractors or the `State<T>` wrapper. State comes from `Depot`, not function parameters of type `State<...>`.
5. **Overusing `Router::new().path(...)`**: it is valid, but `Router::with_path("...")` is clearer for a new route node and matches Salvo's docs/examples. Use `.path(...)` mainly when adding a path filter to an already-built router chain.
6. **Forgetting `.bind().await`** on `TcpListener::new(addr)` — without it, the listener isn't actually bound; you'll get a confusing future-not-Send error.
7. **Using `goal()` when you mean `get()`**: `goal()` matches **regardless of method or sub-path** — handy for `Router::with_path("ws").goal(connect)` (WebSocket upgrade), but a footgun for plain GET endpoints. If unsure, use `.get()`.
8. **Hand-rolling JSON error responses** when `StatusError` does it. Prefer `StatusError` unless the user explicitly wants a custom error envelope.
9. **Pattern-matching `salvo::websocket::Message`**: `match msg { Message::Text(t) => ... }` is **0.65-and-older syntax**. In 0.93 `Message` is an opaque struct — use `msg.is_text()`, `msg.as_str()`, `Message::text(s)`, `Message::binary(v)`. Lots of stale tutorials online still use the enum; ignore them.
10. **Splitting the WebSocket with `futures_util::StreamExt::split`**: works in some versions but the canonical 0.93 path is `ws.recv()` / `ws.send(...)` directly inside the upgrade closure. Reach for `split` only when you genuinely need to drive sender and receiver from separate tasks — usually `tokio::select!` over `recv()` + a `broadcast::Receiver` is simpler.

## Output expectations

- **Code first, prose second.** The user reads the diff faster than your explanation.
- **Cite the Cargo feature** whenever you use a non-`salvo_core` type. Example: "Adds `Compression` middleware (requires `salvo = { features = ["compression"] }`)."
- **Run `cargo check`** if a sandbox is available before claiming success on a non-trivial change. If not, say so explicitly.
- **Write idiomatic Rust**: prefer `?` over `.unwrap()` in handlers, prefer borrowed `&str` extractors when you can, structure routes in modules (`routes::users::router()`) once the app grows past ~5 endpoints.
