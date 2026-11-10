# Cargo features (1.1.1)

The `salvo` crate is a curated re-export over `salvo-core`, `salvo-extra`, `salvo-oapi`, etc. Every non-core capability is gated behind a feature flag. **Forgetting a feature is the most common compile error in AI-generated Salvo code.**

`features = ["full"]` enables most optional Salvo modules and is fine for prototyping. Since **1.1.0** it **does** include `size-limiter` (it didn't in 0.95 – 1.0.1), and since 0.96 also `rfc9457`; it still enables the `aws-lc-rs` TLS backend. For production, prune to what you actually use — it cuts compile time and binary size meaningfully (1.1 also stopped pulling unrelated `salvo_extra` deps such as WebSocket/Tower/request-id into a logging-only build).

The **default** features are `cookie`, `fix-http1-request-uri`, `server`, `server-handle`, `http1`, `http2`, `test`, `aws-lc-rs`, `matched-path` — re-add the ones you need when you set `default-features = false`.

## Feature → re-exports cheatsheet

| Feature | Unlocks (typical imports) | When you need it |
|---|---|---|
| `cookie` | `Cookie`, `CookieJar`, `Response::with_cookies` | Anything reading or setting cookies |
| `affix-state` | `affix_state::inject`, `insert` | DI, sharing app state across handlers (almost always on) |
| `logging` | `Logger` middleware | Request logging |
| `compression` | `Compression`, `CompressionAlgo`, `CompressionLevel` | Response compression (gzip/brotli/zstd/deflate support is inside this feature) |
| `serve-static` | `StaticFile`, `StaticDir` | Serving static assets / SPA |
| `cors` | `Cors`, `AllowOrigin`, `AllowHeaders` builders | Browser cross-origin access |
| `csrf` | `Csrf`, `CsrfStore`, `CsrfCipher` | Form-based CSRF protection |
| `jwt-auth` | `JwtAuth`, `JwtAuthDecoder`, `ConstDecoder`, `RsaDecoder`, etc. | JWT bearer auth — defaults to jsonwebtoken's `aws_lc_rs` crypto (needs a C compiler + CMake to build AWS-LC) |
| `jwt-auth-ring` | same symbols as `jwt-auth` | Alternative crypto: jsonwebtoken's RustCrypto provider (no CMake). Use when AWS-LC won't build — but `aws-lc-rs` is a **default** feature (and in `full`), so you also need `default-features = false` + `ring` (see `references/auth-security.md`) |
| `aws-lc-rs` / `ring` | rustls `CryptoProvider` backend for TLS listeners, QUIC and the proxy client | `aws-lc-rs` is on by default; pick `ring` (with `default-features = false`) when AWS-LC can't build. Since 1.0 Salvo passes the chosen provider to rustls explicitly — `salvo::conn::rustls::default_crypto_provider()` returns it |
| `rfc9457` | `Problem`, `PlainProblem`, `NoExtensions` (in prelude) | RFC 9457 Problem Details error responses (+ OpenAPI integration); in `full` since 0.96 |
| `basic-auth` | `BasicAuth`, `BasicAuthValidator` | Basic auth |
| `session` | `SessionHandler`, `Session`, `SessionStore` | Stateful sessions |
| `flash` | `FlashStore`, `Flash` | One-shot redirect-survival messages |
| `rate-limiter` | `RateLimiter`, quota types | Per-IP/route rate limits |
| `concurrency-limiter` | `max_concurrency` / `MaxConcurrency` | Bound in-flight requests |
| `timeout` | `Timeout` middleware | Per-request timeout |
| `caching-headers` | `CachingHeaders`, `Modified` | Conditional GET (ETag, If-Modified-Since) |
| `cache` | `Cache`, `CacheStore`, `CacheIssuer` | Response cache middleware |
| `websocket` | `WebSocketUpgrade`, `Message` | WebSocket endpoints |
| `sse` | `SseEvent`, `SseKeepAlive`, `sse::stream` | Server-sent events |
| `proxy` | `Proxy`, `ProxyClient` | Reverse proxy / API gateway |
| `oapi` | `#[endpoint]`, `OpenApi`, `Scalar`, `SwaggerUi`, `RapiDoc`, `ReDoc`, `ToSchema`, `ToParameters` | OpenAPI generation. The typed extractors (`JsonBody`, `FormBody`, `FormFile(s)`, `QueryParam`, `PathParam`, `HeaderParam`, `CookieParam`) are **core** since 0.96 (`salvo::extract`, no feature needed); `salvo::oapi::extract` re-exports them and `#[endpoint]` auto-registers their schemas. |
| `size-limiter` | `max_size`, `MaxSize` | Body upper-size-hint guard (in `full` since 1.1.0) |
| `force-https` | `ForceHttps` middleware | HTTP→HTTPS redirect |
| `quinn` | HTTP/3 (QUIC) acceptor | HTTP/3 support |
| `http2-cleartext` | h2c support | HTTP/2 without TLS (gRPC behind LB etc.) |
| `rustls` / `openssl` / `native-tls` | TLS acceptors | HTTPS (pick one) |
| `acme` | ACME / Let's Encrypt acceptor | Auto TLS certificates |
| `request-id` | `RequestId` middleware | X-Request-ID header generation |
| `trailing-slash` | `TrailingSlash` middleware | Strip / append trailing slash |
| `catch-panic` | `CatchPanic` middleware | Recover from handler panics |
| `otel` | `salvo::otel::{Tracing, Metrics}` | OpenTelemetry traces + metrics |
| `tus` | tus resumable-upload handler | Resumable / chunked file uploads (tus.io protocol) |
| `unix` | `UnixListener` | Serving on a Unix domain socket |
| `matched-path` | matched route pattern on `Request` | Route-pattern labels for metrics/logs (in default features) |
| `tower-compat` | `TowerLayerCompat` adapter | Use any `tower::Layer` as Salvo middleware |
| `anyhow` / `eyre` | `Writer` impl for `anyhow::Error` / `eyre::Report` | Returning these from handlers |
| `test` | `TestClient`, `ResponseExt` | Integration tests (also enabled by Salvo's default features) |
| `craft` | `#[craft]` macro | Method handlers on structs (advanced) |

## Recommended feature sets

```toml
# REST API + OpenAPI (most common):
salvo = { version = "1.1.1", features = ["oapi", "logging", "affix-state", "cors", "anyhow"] }

# REST + JWT (swap "jwt-auth" for "jwt-auth-ring" if AWS-LC fails to build):
salvo = { version = "1.1.1", features = ["oapi", "logging", "affix-state", "cors", "jwt-auth", "anyhow"] }

# WebSocket service:
salvo = { version = "1.1.1", features = ["websocket", "logging", "affix-state"] }

# SPA host (frontend + API):
salvo = { version = "1.1.1", features = ["oapi", "logging", "affix-state", "serve-static", "compression"] }

# Reverse proxy / gateway:
salvo = { version = "1.1.1", features = ["proxy", "logging", "rate-limiter", "compression"] }

# HTTPS + auto cert:
salvo = { version = "1.1.1", features = ["oapi", "logging", "rustls", "acme"] }

# Test when using `default-features = false` (otherwise already enabled by default):
[dev-dependencies]
salvo = { version = "1.1.1", features = ["test"] }
```

## When you hit "cannot find struct ..."

That's almost always a missing feature. Look the type up in this table; if it's listed, add the feature. If it's not in this table, query Context7 — Salvo occasionally renames things across minor versions.
