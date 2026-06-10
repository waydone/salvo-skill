# Ops: logging, timeouts, shutdown, compression, proxy, HTTP/2/3

## Logging (feature `logging`)

```rust
use salvo::prelude::*;
use salvo::logging::Logger;

tracing_subscriber::fmt()
    .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
    .init();

let app = Service::new(router).hoop(Logger::new());
```

`Logger::new()` emits a tracing event per request: method, path, status, duration. Combine with `RUST_LOG=info,salvo=info,my_app=debug` to control verbosity.

For request IDs (correlation across logs), enable `request-id`:

```rust
use salvo::request_id::RequestId;
let app = Router::new().hoop(RequestId::new()).hoop(Logger::new()).get(home);
```

## Timeouts (feature `timeout`)

```rust
use salvo::timeout::Timeout;
use std::time::Duration;

let app = Router::new()
    .hoop(Timeout::new(Duration::from_secs(30)))
    .push(/* routes */);
```

Slow handlers get a **503 Service Unavailable** by default. You can swap the error via `Timeout::error(...)` (e.g. to a 408 — but note browsers may auto-resend on 408, which is why 503 is the default). Apply per-route for finer granularity (long uploads need a longer timeout than typical API calls).

## Concurrency limit (feature `concurrency-limiter`)

```rust
use salvo::concurrency_limiter::max_concurrency;
let app = Router::new().hoop(max_concurrency(100)).push(/* */);
```

Bounds in-flight requests. When no permit is available, excess requests get a 429 (or a 413 in the body-too-large branch), both with "max concurrency reached".

## Compression (feature `compression`)

```rust
use salvo::compression::Compression;

let comp = Compression::new()
    .enable_gzip(salvo::compression::CompressionLevel::Default)
    .enable_brotli(salvo::compression::CompressionLevel::Default)
    .enable_zstd(salvo::compression::CompressionLevel::Default)
    .min_length(1024)                            // skip tiny responses
    .force_priority(true);

let app = Router::new().hoop(comp).get(home);
```

Salvo 0.93 exposes one top-level `compression` feature; enable only if you actually serve compressible responses — compile time matters.

`min_length` is critical: compressing a 50-byte response loses bytes, doesn't gain them.

## Graceful shutdown

```rust
use salvo::prelude::*;

#[tokio::main]
async fn main() {
    let acceptor = TcpListener::new("0.0.0.0:5800").bind().await;
    let server = Server::new(acceptor);
    let handle = server.handle();

    // Trigger shutdown on Ctrl-C
    tokio::spawn(async move {
        if let Ok(()) = tokio::signal::ctrl_c().await {
            tracing::info!("shutting down");
            handle.stop_graceful(Some(std::time::Duration::from_secs(30)));
        }
    });

    server.serve(router()).await;
}
```

`stop_graceful(Some(timeout))` — wait for in-flight requests, force-close after `timeout`.
`stop_graceful(None)` — wait indefinitely.
`stop_forcible()` — drop everything immediately (no graceful behavior). (Method is `stop_forcible`, not `stop_force`.)

For Kubernetes / systemd, also handle `SIGTERM`:

```rust
let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
tokio::select! {
    _ = tokio::signal::ctrl_c() => {}
    _ = term.recv() => {}
}
handle.stop_graceful(Some(Duration::from_secs(30)));
```

## Response cache (feature `cache`)

Cache whole responses in memory — useful for expensive read-mostly endpoints:

```rust
use salvo::cache::{Cache, MokaStore, RequestIssuer};
use std::time::Duration;

let cache = Cache::new(
    MokaStore::builder()
        .time_to_live(Duration::from_secs(60))
        .build(),
    RequestIssuer::default(),       // cache key from method + path + query
);

let app = Router::new().hoop(cache).get(expensive_list);
```

`RequestIssuer` decides the cache key (configurable: scheme/host/path/query/method); implement `CacheIssuer` for per-user keys. Don't put it in front of authenticated, user-specific responses with the default issuer — everyone would share one entry.

## OpenTelemetry (feature `otel`)

Salvo ships first-party OTel middleware: `salvo::otel::Tracing` (spans per request) and `salvo::otel::Metrics` (request/error counters + duration histogram). Pair with the `opentelemetry` crate (and `opentelemetry-otlp` to export):

```rust
use opentelemetry::global;
use salvo::otel::{Metrics, Tracing};

let router = Router::new()
    .hoop(Metrics::new())                          // argless — uses global meter "salvo"
    .hoop(Tracing::new(global::tracer("my-app")))  // requires a Tracer argument
    .push(/* routes */);
```

**Version-match gotcha:** your own `opentelemetry` dependency must be the **same version** salvo-otel 0.93.0 uses — `opentelemetry = "0.31"`. A newer one (e.g. 0.3x+) compiles as a *second* copy of the crate and fails with `the trait Tracer is not implemented for BoxedTracer` (two different `Tracer` traits in the graph). Check with `cargo tree -i opentelemetry` if you hit it.

Setting up the OTLP exporter/provider is standard `opentelemetry` crate wiring (not Salvo-specific) — see the official `otel-jaeger` / `logging-otlp` examples and verify versions with Context7; the otel crate family moves fast.

## Reverse proxy (feature `proxy`)

```rust
use salvo::proxy::Proxy;

let app = Router::new()
    .push(
        Router::with_path("api/{**rest}")
            .goal(Proxy::use_hyper_client(["http://upstream-1:8080", "http://upstream-2:8080"]))
    );
```

Round-robins across upstreams. For sticky sessions, custom headers, or auth, build a custom `Proxy` config — see the `salvo_proxy` crate docs.

## HTTP/2 and HTTP/3

HTTP/2 over TLS is automatic with `rustls` / `openssl`. To enable HTTP/3 (QUIC):

```toml
features = [..., "rustls", "quinn"]   # quinn = HTTP/3
```

```rust
use salvo::conn::rustls::{Keycert, RustlsConfig};

let cfg = RustlsConfig::new(
    Keycert::new()
        .cert(include_bytes!("../tls/cert.pem").to_vec())
        .key(include_bytes!("../tls/key.pem").to_vec())
);

let acceptor = TcpListener::new("0.0.0.0:443")
    .rustls(cfg.clone())
    .quinn("0.0.0.0:443")
    .bind()
    .await;

Server::new(acceptor).serve(router).await;
```

Browsers require **the same port for HTTP/2 and HTTP/3** to upgrade automatically (via Alt-Svc).

## Force HTTPS (feature `force-https`)

```rust
use salvo::force_https::ForceHttps;
let app = Service::new(router).hoop(ForceHttps::new());
```

Or run a separate HTTP listener on :80 that 301s to the HTTPS one.

## Multiple listeners / Unix sockets

Combine listeners with `.join(...)` — one server, several binds (this is also how ACME HTTP-01 serves :80 + :443, see `auth-security.md`):

```rust
let acceptor = TcpListener::new("0.0.0.0:5800")
    .join(TcpListener::new("0.0.0.0:5801"))
    .bind().await;
```

Unix domain socket (feature `unix`, Unix-only): `salvo::conn::UnixListener::new("/tmp/app.sock").bind().await` — also joinable with TCP listeners.

## Health check pattern

Skip middleware on `/healthz` so logger noise and rate limiters don't interfere:

```rust
#[handler]
async fn healthz() -> &'static str { "ok" }

let app = Router::new()
    .push(Router::with_path("healthz").get(healthz))    // no hoops
    .push(
        Router::new()
            .hoop(Logger::new())
            .hoop(rate_limiter)
            .push(/* real routes */)
    );
```

## Observability checklist

For a production-ready Salvo service:

1. `tracing-subscriber` initialized with `EnvFilter`.
2. `Logger` middleware on the main router (skipped for health endpoints).
3. `RequestId` middleware to correlate logs.
4. Graceful shutdown handling SIGTERM + SIGINT.
5. `Timeout` middleware for slow-request defense.
6. `Compression` for client bandwidth.
7. `RateLimiter` on public endpoints.
8. `Catcher` for uniform 4xx/5xx error pages.
9. `otel` feature (`Tracing` + `Metrics` middleware) if you export to an OTel collector.
10. `cargo build --release` before deploying — debug builds are 5–10× slower.
