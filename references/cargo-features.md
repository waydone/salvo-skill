# Cargo features (0.93.0)

The `salvo` crate is a curated re-export over `salvo-core`, `salvo-extra`, `salvo-oapi`, etc. Every non-core capability is gated behind a feature flag. **Forgetting a feature is the most common compile error in AI-generated Salvo code.**

`features = ["full"]` enables everything. Use it for prototyping. For production, prune to what you actually use — it cuts compile time and binary size meaningfully.

## Feature → re-exports cheatsheet

| Feature | Unlocks (typical types in `salvo::prelude`) | When you need it |
|---|---|---|
| `cookie` | `Cookie`, `CookieJar`, `Response::with_cookies` | Anything reading or setting cookies |
| `affix-state` | `affix_state::inject`, `insert` | DI, sharing app state across handlers (almost always on) |
| `logging` | `Logger` middleware | Request logging |
| `compression` | `Compression`, `CompressionAlgo`, `CompressionLevel` (gzip default) | Response compression |
| `compression-brotli` / `-gzip` / `-zstd` / `-deflate` | algorithm-specific encoders | Picking specific compression backends |
| `serve-static` | `StaticFile`, `StaticDir` | Serving static assets / SPA |
| `cors` | `Cors`, `CorsLayer` builders | Browser cross-origin access |
| `csrf` | `Csrf`, `CsrfStore`, `CsrfCipher` | Form-based CSRF protection |
| `jwt-auth` | `JwtAuth`, `JwtAuthDecoder`, `ConstDecoder`, `RsaDecoder`, etc. | JWT bearer auth |
| `basic-auth` | `BasicAuth`, `BasicAuthValidator` | Basic auth |
| `session` | `SessionHandler`, `Session`, `SessionStore` | Stateful sessions |
| `flash` | `FlashStore`, `Flash` | One-shot redirect-survival messages |
| `rate-limiter` | `RateLimiter`, quota types | Per-IP/route rate limits |
| `concurrency-limiter` | `ConcurrencyLimiter` | Bound in-flight requests |
| `timeout` | `Timeout` middleware | Per-request timeout |
| `caching-headers` | `CachingHeaders`, `Modified` | Conditional GET (ETag, If-Modified-Since) |
| `cache` | `Cache`, `CacheStore`, `CacheIssuer` | Response cache middleware |
| `ws` / `websocket` | `WebSocketUpgrade`, `Message` | WebSocket endpoints |
| `sse` | `SseEvent`, `SseKeepAlive`, `sse::stream` | Server-sent events |
| `proxy` | `Proxy`, `ProxyClient` | Reverse proxy / API gateway |
| `oapi` | `#[endpoint]`, `OpenApi`, `Scalar`, `SwaggerUi`, `Rapidoc`, `Redoc`, `ToSchema`, `ToParameters`, `Json`, `Form`, `JsonBody`, `FormBody`, `QueryParam`, `PathParam`, `HeaderParam`, `CookieParam` | OpenAPI generation. Note: with `oapi`, prefer the typed extractors over `req.parse_*` — they auto-register schemas. |
| `force-https` | `ForceHttps` middleware | HTTP→HTTPS redirect |
| `quinn` / `http3` | HTTP/3 (QUIC) acceptor | HTTP/3 support |
| `rustls` / `openssl` / `native-tls` | TLS acceptors | HTTPS (pick one) |
| `acme` | ACME / Let's Encrypt acceptor | Auto TLS certificates |
| `request-id` | `RequestId` middleware | X-Request-ID header generation |
| `trailing-slash` | `TrailingSlash` middleware | Strip / append trailing slash |
| `force-host` | `ForceHost` middleware | Canonical host redirect |
| `catch-panic` | `CatchPanic` middleware | Recover from handler panics |
| `tower-compat` | `TowerLayerCompat` adapter | Use any `tower::Layer` as Salvo middleware |
| `anyhow` / `eyre` | `Writer` impl for `anyhow::Error` / `eyre::Report` | Returning these from handlers |
| `test` | `TestClient`, `ResponseExt` | Integration tests |
| `craft` | `#[craft]` macro | Method handlers on structs (advanced) |

## Recommended feature sets

```toml
# REST API + OpenAPI (most common):
salvo = { version = "0.93.0", features = ["oapi", "logging", "affix-state", "cors", "anyhow"] }

# REST + JWT:
salvo = { version = "0.93.0", features = ["oapi", "logging", "affix-state", "cors", "jwt-auth", "anyhow"] }

# WebSocket service:
salvo = { version = "0.93.0", features = ["websocket", "logging", "affix-state"] }

# SPA host (frontend + API):
salvo = { version = "0.93.0", features = ["oapi", "logging", "affix-state", "serve-static", "compression-gzip", "compression-brotli"] }

# Reverse proxy / gateway:
salvo = { version = "0.93.0", features = ["proxy", "logging", "rate-limiter", "compression-gzip"] }

# HTTPS + auto cert:
salvo = { version = "0.93.0", features = ["oapi", "logging", "rustls", "acme"] }

# Test (always dev-only):
[dev-dependencies]
salvo = { version = "0.93.0", features = ["test"] }
```

## When you hit "cannot find struct ..."

That's almost always a missing feature. Look the type up in this table; if it's listed, add the feature. If it's not in this table, query Context7 — Salvo occasionally renames things across minor versions.
