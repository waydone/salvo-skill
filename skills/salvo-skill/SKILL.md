---
name: salvo-skill
description: "Salvo Rust web framework specialist (target 1.1.2). USE whenever ANY apply. (1) ANY mention of 'salvo'/'Salvo', even casual ('Salvo 项目报 404', 'use salvo::', 'salvo = 1.1', 'salvo = 0.95'). (2) Cargo.toml has salvo = '…', or source imports salvo/salvo_core/salvo_extra/salvo_oapi. (3) Salvo symbols — #[handler], #[endpoint], Router::with_path, Depot, JwtAuth, WebSocketUpgrade, SwaggerUi, affix_state::inject, StatusError. (4) Fresh Rust web/API project (REST, WebSocket, SSE, reverse proxy, OpenAPI, JWT, internal dashboard, LLM streaming, real-time push) where user does NOT pick axum/actix/rocket/warp/poem — Salvo is this user's default. (5) Salvo debugging — route 404, missing JwtAuth, Cargo feature missing, Message::Text fails, angle-bracket path, missing Writer impl, empty OpenAPI / QUERY route missing from docs, deprecated Depot::obtain/inject warnings, AWS-LC build failure on jwt-auth, otel metrics vanished after upgrade, ACME stuck on first cert, multipart 413, E0433 'cannot find RefOr in salvo' on #[endpoint(request_body)], 400 on %2F paths. (6) Migrating TO Salvo or upgrading 0.9x → 1.x. SKIP axum/actix/rocket/warp/poem when staying inside, hyper/tower direct, non-Rust stacks, jsonwebtoken without web. Bundles 1.1.2 API + Context7 rule for non-trivial code. New Cargo.toml: edition = '2024', never 2021."
---

# Salvo Skill (target: 1.1.2)

You are helping the user build Rust web services with **Salvo** — a modular, ergonomic Rust web framework. The user develops AI-first: they expect you to make sound default choices and produce production-grade code without round-tripping for clarification on every detail.

This skill encodes Salvo's 1.1.2 API surface and a workflow for keeping that surface accurate as the library evolves.

**Edition rule — applies to every `Cargo.toml` you write, Salvo or not.** A new crate is `edition = "2024"`. That is the current edition (2015 → 2018 → 2021 → **2024**; nothing newer exists on Rust 1.100), it is what `cargo new` / `cargo init` write, and Salvo itself is built on it. `edition = "2021"` in a fresh manifest is a stale training-data habit, not a conservative default — don't write it. The only time an older edition is right is an **existing** project whose `Cargo.toml` already declares it: match that, and treat a migration (`cargo fix --edition`) as its own task. Prefer `cargo new` over hand-writing the manifest when you can run commands.

### What changed 0.95.2 → 1.1.2 (read this first if you know the 0.9x API)

Salvo hit **1.0** on 2026-09-24, but for application code it is an evolution, not a rewrite. Handlers, routing, `Writer`/`Scribe`, `Depot`, `affix_state`, WebSocket `Message`, `Proxy` and `TestClient` keep their shapes, and **every 0.94 deprecated alias still compiles** (1.0 removed none of them — checked in 1.1.1 source). The whole `full` stack plus every snippet in this skill was `cargo check`-verified on 1.1.2 with Rust 1.100. The changes that actually touch code you'd write:

- **MSRV is now Rust 1.95** (1.1.0; 0.95 was 1.94). `rustup update stable` if you see an MSRV error.
- **Typed extractors moved into core (0.96).** `JsonBody`, `FormBody`, `FormFile`, `FormFiles`, `PathParam`, `QueryParam`, `HeaderParam`, `CookieParam` now live in `salvo_core::extract` → **`use salvo::extract::{...}`**, and they need **no `oapi` feature**. `salvo::oapi::extract::{...}` still compiles (a re-export of the same types). They are still **not** in the prelude.
- **`full` now includes `size-limiter`** (1.1.0) and `rfc9457` (0.96). The default features are unchanged (`aws-lc-rs` is still on by default).
- **OpenTelemetry (1.0, breaks dashboards):** needs `opentelemetry = "0.33"` (was 0.32). Metrics follow the HTTP semconv: `http.server.request.duration` in **seconds**; `salvo_request_duration_ms` / `salvo_request_count` / `salvo_error_count` are gone; `http.route` replaces `url.full`. Spans are named `{method} {route}`. Build `Metrics::new()` **after** `global::set_meter_provider(...)`, or it binds to the no-op provider. → `references/ops.md`
- **OpenAPI:** a document that declares 3.2 now **emits `QUERY` and custom-method routes** (1.0); a 3.1 document skips them with a `warn!`. 1.1 breaks: `Parameters(vec)` → `Parameters::from(vec)` (it is now `Vec<RefOr<Parameter>>`); `Header::schema` is `Option<RefOr<Schema>>`; new enum variants (`PathItemType::Query`, `ParameterIn::QueryString`, …) break exhaustive `match`es. → `references/openapi.md`
- **TLS (1.0):** Salvo hands rustls an explicit `CryptoProvider`, so the *"Could not automatically determine the process-level CryptoProvider"* panic is gone. Share the provider with other rustls crates via `salvo::conn::rustls::default_crypto_provider()`. 1.1 adds `req.tls_info()` (mTLS client certs, SNI, ALPN).
- **ACME HTTP-01 (1.1):** `listener.bind().await` now waits for the first certificate, so serve the challenge router on :80 **before** binding. The old `http01_challenge(&mut router)` + `.join(TcpListener :80)` pattern hangs on first issuance. → `references/auth-security.md`
- **Secure-by-default behavior changes:**
  - Static SVG/XML are served as `attachment` + `nosniff` (1.0). For trusted directories, opt back in with `.disposition_type("inline")`.
  - Multipart is capped at **128 parts / 32 files → 413** (1.1). Tune it with `salvo::http::FormLimits`.
  - `BasicAuth` reads only `Authorization` (1.1).
  - CSRF validates every unsafe method, including CONNECT and extension methods (1.1).
  - `HeaderFinder::default()` now actually authenticates (1.1).
  - WebSocket rejects non-GET handshakes (1.1).
- **Smaller 1.1 breaks:** the JWT finders' `allowed_methods` **field** is `Option<Vec<Method>>` (the builder is unchanged); tus `Metadata` values are `Option<Vec<u8>>` (use `get_text`); sessions run on `saysion` 0.4 (1.1.1). Dropping the `serve` future now cancels in-flight connections, so drain with `ServerHandle::stop_graceful`.
- **1.1.2 (2026-11-10) — a hardening patch with one regression and one field break:**
  - ⚠️ **Regression (in the `salvo-oapi-macros` 1.1.2 sub-crate — a `cargo update` pulls it into a `salvo = "1.1.1"` project too):** `#[endpoint(request_body(...))]` no longer compiles through the `salvo` facade — `E0433: cannot find RefOr in salvo`. Let a `JsonBody<T>` argument declare the body instead, or add `salvo-oapi = "=1.1.2"` as a direct dependency. → `references/openapi.md`
  - `Operation.request_body` is now `Option<RefOr<RequestBody>>` so a body can be a `$ref`. The `.request_body(...)` builder takes an inline body or a `Ref`; a direct field assignment (e.g. in a custom `EndpointArgRegister`) needs `.into()`.
  - **Ambiguous paths get 400 before routing:** an encoded separator (`%2F`, `%5C`), a literal backslash or a NUL byte in the path is rejected, and dot segments are normalized before matching. A path parameter can no longer carry an encoded slash. → `references/routing.md`
  - Response cache: a request `Cache-Control: no-cache` / `no-store` / `max-age` / `min-fresh` (or `Pragma: no-cache`) bypasses lookup and storage, and keys are isolated by a validated `Host`. Compression leaves the body alone when the client ranks `identity` above every codec. → `references/ops.md`
  - WebSocket sessions and proxied upgrade tunnels now hold their `max_connections` permit and are drained by `stop_graceful`. → `references/realtime.md`
  - New opt-ins: `CookieStore::host_prefixed()` for CSRF (`__Host-` cookie), `max_concurrency(n).include_body(true)`, `OidcDecoder::revalidation_interval(...)`.
- **New capabilities:**
  - RFC 9457 `Problem` responses plus a `ProblemGoal` catcher (`rfc9457`, in `full`) → `references/error-handling.md`.
  - Automatic **405 + `Allow`** when the path matches but the method doesn't (1.1).
  - Owned `FormFile` / `FormFiles` keep the upload alive after the request ends (1.1).
  - Cookie extractors accept JSON values (1.0): `CookieParam<T>` falls back to JSON, and `Extractible` takes `source(from = "cookie", parse = "json")`.

#### Still current from the 0.94/0.95 round (if you're coming from ≤0.93)

- **`Depot` type-keyed accessors were renamed** (the old names remain as `#[deprecated(since = "0.94.0")]` aliases). `obtain::<T>()` → `get_typed::<T>()`, `obtain_mut` → `get_typed_mut`, `inject(v)` → `insert_typed(v)`, `scrape::<T>()` → `remove_typed::<T>()`, `contains::<T>()` → `contains_typed::<T>()`.
- **String-keyed `Depot`:** `insert("k", v)` and `get::<T>("k")` are unchanged, but `remove` changed shape. 0.93's `depot.remove::<T>("k")` is now `depot.remove(key) -> Option<Box<dyn Any + Send + Sync>>`, which is a hard compile break. `delete(key)` is deprecated; use `remove(key).is_some()`. `affix_state::inject(...)` / `AffixList::inject(...)` are a different, **un-deprecated** API — leave them.
- **JWT crypto backend:** `jwt-auth` uses jsonwebtoken's `aws_lc_rs` (needs a C compiler + CMake). `jwt-auth-ring` uses RustCrypto. `aws-lc-rs` is also a *default* `salvo` feature, so dropping AWS-LC entirely takes `default-features = false` + `ring`. → `references/auth-security.md`
- **`Server`:** `stop_forcible()` → `stop_forceful()`. Also new: `max_connections(n)` and `fuse_config(...)` / `disable_fuse()`.
- **`Response`:** `stuff(status, v)` → `render_with_status(status, v)`. `Json<T>` **replaces** an already-buffered body (text scribes still append).
- **Other renames:** `Scheme/Host/PortFilter::lack` → `fallback`; `StatusError::request_header_fields_toolarge` → `…_too_large`, `unavailable_for_legalreasons` → `…_legal_reasons`; oapi `Parameter::parameter_in` → `location`.
- **Still available:** `Router::query(h)` (HTTP `QUERY`); `ToSchema` for `OsString` / `PathBuf` (schema only — not extractors, so no `PathParam<PathBuf>`).

If a future release changes this surface again, re-verify with Context7 and bump this section. Latest stable as of this skill: **1.1.2** (2026-11-10).

## Hard rule: verify with Context7 before non-trivial code

Salvo's API has changed meaningfully across recent versions (path syntax, OpenAPI macros, middleware crate splits, the 0.94 `Depot`/JWT changes, the 0.96 extractor move and the 1.0/1.1 changes above). Your training data is older than 1.1.2.

**Before writing any non-trivial Salvo code, query Context7.** Non-trivial means anything beyond `fn main` + `Router::new().get(hello)`.

Use this exact pattern:

```
Tool: mcp__plugin_context7_context7__query-docs
libraryId: /websites/rs_salvo
query: <a specific question, e.g. "Salvo 1.1 JwtAuth middleware constructor and decoder configuration">
```

Fallback library IDs if the first returns nothing useful:
- `/salvo-rs/salvo` — GitHub source (smaller corpus, sometimes fresher)
- `/websites/salvo_rs_zh-hans` — Chinese-language docs (use if user writes in Chinese and a translated example would help)

Skip Context7 only when the user is just asking conceptual questions ("what is Salvo?") or when the snippet you need is already verbatim in `references/` and you're sure it matches 1.1.2.

If Context7 errors out, tell the user and fall back to your knowledge — but flag the API surface as unverified and recommend running `cargo check` before trusting it.

## Project skeleton — paste-ready

This is the minimum that compiles on 1.1.2. Use it as the starting point for any new app.

`Cargo.toml`:

```toml
[package]
name = "myapp"
version = "0.1.0"
edition = "2024"   # the current edition — never "2021" in a new project (see the edition rule at the top). Salvo 1.1's MSRV is Rust 1.95; current stable is 1.100.

[dependencies]
salvo = { version = "1.1.2", features = ["full"] } # "full" includes size-limiter since 1.1.0
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
serde = { version = "1", features = ["derive"] }
tracing = "0.1"
tracing-subscriber = "0.3"
```

`features = ["full"]` is fine for prototyping (since 1.1.0 it also covers `size-limiter`). For production, switch to a curated list — see `references/cargo-features.md`. Common minimal sets:

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

In addition, the macro accepts **typed extractors** as parameters (`JsonBody<T>`, `FormBody<T>`, `FormFile`, `FormFiles`, `PathParam<T>`, `QueryParam<T, REQUIRED>`, `HeaderParam<T, REQUIRED>`, `CookieParam<T, REQUIRED>` — import from `salvo::extract`). These can appear alongside the context parameters above.

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

Method shortcuts on `Router`: `.get(h)`, `.post(h)`, `.put(h)`, `.patch(h)`, `.delete(h)`, `.head(h)`, `.options(h)`, and `.query(h)` (since 0.95, for the HTTP `QUERY` method). A path that matches with the wrong method gets an automatic **405 + `Allow`** (1.1). Use `.goal(h)` when **only this handler** should match (e.g. WebSocket upgrade routes — see `references/realtime.md`).

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
use salvo::extract::{JsonBody, PathParam, QueryParam}; // core since 0.96 — NOT in prelude, no `oapi` needed

#[handler]
async fn create(json: JsonBody<NewUser>) -> Json<User> {
    let NewUser { name, email } = json.into_inner();
    /* ... */
}
```

Available: `JsonBody<T>`, `FormBody<T>`, `FormFile`, `FormFiles`, `QueryParam<T, REQUIRED>`, `PathParam<T>`, `HeaderParam<T, REQUIRED>`, `CookieParam<T, REQUIRED>` — all in **`salvo::extract`** (core; works without `oapi`), **not** in `salvo::prelude`. `salvo::oapi::extract::{...}` re-exports the same types, so older imports keep compiling; under `#[endpoint]` either path registers schemas.

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
    let pool = depot.get_typed::<PgPool>().map_err(|_| StatusError::internal_server_error())?;
    /* ... */
}
```

`depot.get_typed::<T>()` looks up by **type** (this is the 0.94 name; the old `obtain::<T>()` still compiles but is deprecated). `depot.get::<&str>("key")` and `depot.insert("key", val)` use string keys — handy for per-request data, and unchanged.

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
| `references/handlers.md` | Return-type quirks, `Writer` impls, streaming responses, `#[craft]` struct-method handlers |
| `references/data-extraction.md` | File uploads, multipart, validation, custom extractors |
| `references/middleware-state.md` | Writing custom middleware, `FlowCtrl`, DI patterns, scoped state |
| `references/error-handling.md` | Custom error envelopes, `Catcher`, 404 pages |
| `references/auth-security.md` | JWT, basic auth, sessions, CSRF, CORS, rate limiting, TLS/ACME |
| `references/openapi.md` | `#[endpoint]`, `ToSchema`, `ToParameters`, SwaggerUi / Scalar / RapiDoc / ReDoc setup |
| `references/realtime.md` | WebSocket, SSE, broadcast patterns |
| `references/database.md` | SQLx / SeaORM / Diesel integration via `affix_state` |
| `references/files.md` | Static directories, embedded assets (rust-embed), file uploads, size limits, tus resumable uploads |
| `references/ops.md` | Logging/tracing, timeout, graceful shutdown, compression, proxy, HTTP/2/3, OpenTelemetry, response cache, Unix socket / multi-listener |
| `references/testing.md` | `TestClient`, integration tests, mocking middleware |

If the user's question doesn't fit a single file, read 1–2 candidates and synthesize. Don't read all of them speculatively.

## Anti-patterns to avoid

These come up repeatedly in AI-generated Salvo code. Catch them in your own output before submitting.

1. **Old path syntax**: `with_path("articles/<id>")` — wrong since 0.76. Use `{id}`.
2. **Forgetting Cargo features**: writing `JwtAuth::new(...)` without `features = ["jwt-auth"]` (or `"full"`) → "cannot find struct" compile error. When you reach for a non-core type, mention the feature flag inline. (`jwt-auth` defaults to the `aws-lc-rs` crypto backend, which needs a C toolchain; use `jwt-auth-ring` if that build fails.)
3. **Returning raw `anyhow::Error`** without enabling the `anyhow` feature → no `Writer` impl, won't compile.
4. **Mixing `axum`/`actix` idioms**: Salvo handlers don't use tuple extractors or the `State<T>` wrapper. State comes from `Depot`, not function parameters of type `State<...>`.
5. **Overusing `Router::new().path(...)`**: it is valid, but `Router::with_path("...")` is clearer for a new route node and matches Salvo's docs/examples. Use `.path(...)` mainly when adding a path filter to an already-built router chain.
6. **Forgetting `.bind().await`** on `TcpListener::new(addr)` — without it, the listener isn't actually bound; you'll get a confusing future-not-Send error.
7. **Using `goal()` when you mean `get()`**: `goal()` matches **regardless of method or sub-path** — handy for `Router::with_path("ws").goal(connect)` (WebSocket upgrade), but a footgun for plain GET endpoints. If unsure, use `.get()`.
8. **Hand-rolling JSON error responses** when `StatusError` does it. Prefer `StatusError` unless the user explicitly wants a custom error envelope.
9. **Pattern-matching `salvo::websocket::Message`**: `match msg { Message::Text(t) => ... }` is **0.65-and-older syntax**. In 0.9x and 1.x `Message` is an opaque struct — use `msg.is_text()`, `msg.as_str()`, `Message::text(s)`, `Message::binary(v)`. Lots of stale tutorials online still use the enum; ignore them.
10. **Splitting the WebSocket with `futures_util::StreamExt::split`**: works in some versions but the canonical path (0.95 → 1.1) is `ws.recv()` / `ws.send(...)` directly inside the upgrade closure. Reach for `split` only when you genuinely need to drive sender and receiver from separate tasks — usually `tokio::select!` over `recv()` + a `broadcast::Receiver` is simpler.
11. **Deprecated `Depot` accessors**: the *old* type-keyed names `depot.obtain::<T>()` / `depot.inject(v)` / `depot.scrape::<T>()` / `depot.contains::<T>()` still compile but warn since 0.94. Write the new ones — `get_typed` / `insert_typed` / `remove_typed` / `contains_typed`. (Don't "fix" `affix_state::inject(...)` — that's a separate, current, un-deprecated API.)
12. **ACME HTTP-01 via `.join(TcpListener :80)`** — the pre-1.1 pattern. Since 1.1 `bind()` waits for the first certificate, so the joined :80 side never serves the challenge and issuance hangs. Spawn a separate :80 `Server` with the challenge router **before** `listener.bind().await` (see `references/auth-security.md`).
13. **Stale otel assumptions**: old metric names (`salvo_request_duration_ms` in **ms**), `opentelemetry = "0.32"`, or `Metrics::new()` constructed before `global::set_meter_provider` (binds to no-op, records nothing). 1.x: `opentelemetry = "0.33"`, `http.server.request.duration` in **seconds** (see `references/ops.md`).
14. **`Parameters(vec)` in hand-built OpenAPI** — 1.1 made it `Vec<RefOr<Parameter>>`; write `Parameters::from(vec)`.
15. **`edition = "2021"` in a new `Cargo.toml`** — stale habit. New crates are `edition = "2024"`; only an existing project keeps what it already declares.
16. **`request_body(...)` inside `#[endpoint(...)]` on 1.1.2** — fails with `E0433: cannot find RefOr in salvo`. Declare the body with a `JsonBody<T>` / `FormBody<T>` argument, or add `salvo-oapi = "=1.1.2"` as a direct dependency (see `references/openapi.md`).
17. **Encoded slashes in a path segment** (`/files/a%2Fb`) — 400 since 1.1.2. Capture the remainder with `{**rest}` or move the value to the query string.

## Output expectations

- **Code first, prose second.** The user reads the diff faster than your explanation.
- **Cite the Cargo feature** whenever you use a non-`salvo_core` type. Example: "Adds `Compression` middleware (requires `salvo = { features = ["compression"] }`)."
- **Run `cargo check`** if a sandbox is available before claiming success on a non-trivial change. If not, say so explicitly.
- **Write idiomatic Rust**: prefer `?` over `.unwrap()` in handlers, prefer borrowed `&str` extractors when you can, structure routes in modules (`routes::users::router()`) once the app grows past ~5 endpoints.
