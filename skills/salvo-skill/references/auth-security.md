# Auth & security

## JWT (feature `jwt-auth`)

Salvo's JWT middleware extracts a token from the request, decodes it, and stores the claims in `Depot`. Failed auth either rejects (default) or marks the request as unauthenticated and continues (if `force_passed = true`).

```rust
use salvo::prelude::*;
use salvo::jwt_auth::{ConstDecoder, HeaderFinder, QueryFinder, CookieFinder, JwtAuth, JwtAuthState};
use serde::{Deserialize, Serialize};
use jsonwebtoken::{EncodingKey, Header, Algorithm};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Claims {
    sub: String,
    exp: i64,
}

const SECRET: &[u8] = b"please-change-me";

fn jwt_middleware() -> JwtAuth<Claims, ConstDecoder> {
    JwtAuth::new(ConstDecoder::from_secret(SECRET))
        .finders(vec![
            Box::new(HeaderFinder::new()),                          // Authorization: Bearer ...
            Box::new(QueryFinder::new("token")),                    // ?token=...
            Box::new(CookieFinder::new("jwt")),                     // Cookie: jwt=...
        ])
        .force_passed(false)
}

#[handler]
async fn me(depot: &mut Depot) -> Result<Json<Claims>, StatusError> {
    let data = depot.jwt_auth_data::<Claims>()
        .ok_or_else(StatusError::unauthorized)?;
    Ok(Json(data.claims.clone()))
}

#[handler]
async fn login(req: &mut Request) -> Result<String, StatusError> {
    let exp = (chrono::Utc::now() + chrono::Duration::hours(24)).timestamp();
    let claims = Claims { sub: "user-1".into(), exp };
    let token = jsonwebtoken::encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(SECRET),
    ).map_err(|_| StatusError::internal_server_error())?;
    Ok(token)
}

let app = Router::new()
    .push(Router::with_path("login").post(login))
    .push(Router::with_path("api").hoop(jwt_middleware()).get(me));
```

`depot.jwt_auth_state()` returns one of `JwtAuthState::Authorized` / `Unauthorized` / `Forbidden`. Useful when `force_passed = true` and you want to inspect inside the handler.

1.1 behavior changes worth knowing:
- `HeaderFinder::default()` now matches `HeaderFinder::new()` (all methods, `Authorization` header) — before 1.1 the `Default` impl silently rejected every token, so code that used `..Default::default()` / `HeaderFinder::default()` starts authenticating.
- The finders' public **`allowed_methods` field is now `Option<Vec<Method>>`** (`None` = every method, including extension methods). The `.allowed_methods(vec![...])` builder is unchanged; only direct field assignment needs `Some(...)`. An explicit empty list still disables extraction.
- Each `JwtAuth` attempt **replaces** the depot's JWT data/state/token/error, so with several `JwtAuth` layers a later failure can no longer leave an earlier layer's claims behind.

For RSA / ECDSA decoders use `RsaDecoder`, `EcdsaDecoder`. For dynamic keys (e.g. JWKS rotation), implement `JwtAuthDecoder` yourself.

### Crypto backend (0.94+): `jwt-auth` vs `jwt-auth-ring`

Since `jsonwebtoken` went to v11, the JWT stack must pick a crypto provider, and Salvo surfaces the choice as two top-level features:

- `features = ["jwt-auth"]` → jsonwebtoken's **`aws_lc_rs`** provider (the default; also what `full` pulls in). AWS-LC compiles from C via CMake, so the build host needs a C compiler **and** CMake. On a normal dev machine with build tools this just works — the whole `full` stack `cargo check`s clean.
- `features = ["jwt-auth-ring"]` → jsonwebtoken's **`rust_crypto`** (RustCrypto) provider, plus `ring` for any rustls TLS. Reach for this when the AWS-LC build fails — typically slim containers, cross-compilation, or CI images without CMake. Symptom to recognize: a build error deep in `aws-lc-sys` / `cmake`, not in your own code. (`ring` still has a little C but needs no CMake; the JWT crypto itself is pure-Rust RustCrypto.)

Same `JwtAuth` API either way — only the Cargo feature differs.

⚠️ **`aws-lc-rs` is a *default* feature of `salvo`, and `full` lists it too.** So swapping `jwt-auth` → `jwt-auth-ring` steers *JWT* to RustCrypto but does **not** remove AWS-LC from the graph. If AWS-LC itself is the thing that won't build, turn the defaults off and pick the ring backend explicitly — `salvo = { version = "1.1.2", default-features = false, features = ["server", "http1", "http2", "jwt-auth-ring", "rustls", "ring", ...] }` (re-add any other defaults you use: `cookie`, `matched-path`, `test`, …). Check with `cargo tree -i aws-lc-rs` — it should report nothing.

**rustls provider selection (1.0+):** Salvo now builds every rustls config with an **explicit** `CryptoProvider` chosen from its own `aws-lc-rs` / `ring` features, so the old runtime panic *"Could not automatically determine the process-level `CryptoProvider`"* (from `RustlsListener` / `QuinnListener` / `AcmeListener` / the proxy's `HyperClient` when another crate enabled the other rustls backend) is gone. If you call `CryptoProvider::install_default` yourself, Salvo uses that. To make other rustls-based crates agree with Salvo, pass them `salvo::conn::rustls::default_crypto_provider()`.

**jsonwebtoken still can't auto-select with both JWT providers enabled.** If both `jwt-auth` and `jwt-auth-ring` end up on (e.g. via a workspace member), call `salvo::jwt_auth::install_crypto_provider()` at the start of `main`, or token validation panics at runtime.

**OIDC / JWKS (`OidcDecoder`, 1.1.2):** cached provider keys now have a bounded lifetime that follows the JWKS response's `Cache-Control` (`max-age`, `stale-while-revalidate`, `stale-if-error`, each capped at 30 days; with no directives a key is reused for at most ~61 s). `no-cache` / `no-store` on the JWKS response means a network validation per token, rate-limited by a shared one-second cooldown during which lookups **fail closed** — if your IdP really sends those, opt out with `OidcDecoder::new(issuer, audience).await?.revalidation_interval(Duration::ZERO)`. A removed key stays valid until its cached copy expires, so short `max-age` on the IdP side is what bounds revocation. Verify the builder with Context7 before use.

## Basic auth (feature `basic-auth`)

```rust
use salvo::prelude::*;
use salvo::basic_auth::{BasicAuth, BasicAuthValidator};

struct Validator;

// No #[async_trait] — BasicAuthValidator uses native RPITIT (async fn in trait) (still in 1.1.2); adding it fails E0195.
impl BasicAuthValidator for Validator {
    async fn validate(&self, username: &str, password: &str, _depot: &mut Depot) -> bool {
        username == "admin" && password == "secret"
    }
}

let auth = BasicAuth::new(Validator);
let app = Router::new().hoop(auth).get(home);
```

Since 1.1 `BasicAuth` reads **only `Authorization`** by default, so a forward proxy's `Proxy-Authorization` credentials can't authenticate against your origin. If you deliberately accept proxy credentials, opt in with `BasicAuth::new(v).set_header_names(vec![header::AUTHORIZATION, header::PROXY_AUTHORIZATION])`.

## Sessions (feature `session`)

```rust
use salvo::session::{SessionHandler, MemoryStore, SessionDepotExt};
// or RedisStore from salvo-session crate / community store

let store = MemoryStore::new();
let session_secret = [7_u8; 64]; // minimum 64 bytes; shorter keys panic
let session = SessionHandler::builder(store, &session_secret)
    .cookie_name("sid")
    .build()?;

let app = Router::new().hoop(session).get(home);

#[handler]
async fn login(depot: &mut Depot) -> Result<&'static str, StatusError> {
    let s = depot.session_mut().ok_or_else(StatusError::internal_server_error)?;
    s.insert("user_id", 42)                       // -> Result<(), serde_json::Error>
        .map_err(|_| StatusError::internal_server_error())?;
    Ok("logged in")
}
```

Sessions are backed by **`saysion` 0.4** since 1.1.1 (was 0.2): regenerating a session now revokes the previous server-side ID, and `MemoryStore` rejects stale saves after rotation/logout. If you use a `saysion` store type directly (custom/Redis store), depend on the **same** `saysion` version as `salvo-session`, or the store won't satisfy the trait.

## CORS (feature `cors`)

```rust
use salvo::cors::{Cors, AllowOrigin};
use salvo::http::Method;

let cors = Cors::new()
    .allow_origin(["https://app.example.com"])                     // array OK for origin
    .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
    .allow_headers(vec!["authorization", "content-type"])          // headers needs Vec<&str>, NOT an array
    .allow_credentials(true)
    .max_age(3600)
    .into_handler();

let app = Service::new(router).hoop(cors);
```

For dev (`*` origin), use `.allow_origin(AllowOrigin::any())` (there is no `Cors::any_origin()`), or the shortcuts `Cors::permissive()` / `Cors::very_permissive()`. CORS preflight handling (OPTIONS) is automatic.

## CSRF (feature `csrf`)

```rust
use salvo::csrf::*;

let csrf = Csrf::new(
    BcryptCipher::new(),
    CookieStore::new(),
    FormFinder::new("csrf_token"),
);

let app = Router::new().hoop(csrf).get(form_page).post(submit);
```

The default skipper is `req.method().is_safe()`: it skips `GET` / `HEAD` / `OPTIONS` / `TRACE` and validates **every** other method — `POST` / `PUT` / `PATCH` / `DELETE`, and (since 1.1) also `CONNECT` and extension methods. If saving a new or rotated token fails, the middleware now returns **500** instead of running your handler with an unsaved token. In templates, read `depot.csrf_token()` (via `CsrfDepotExt`) and put it in a hidden form field named to match the finder.

**Prefer `CookieStore::host_prefixed()` on HTTPS sites (1.1.2).** It issues the token as `__Host-salvo.csrf` with `Secure`, `HttpOnly`, `Path=/` and no `Domain`, which browsers refuse to let a sibling subdomain overwrite — closing the cookie-injection hole of the plain double-submit cookie. `Secure` is set explicitly, so it also works behind a TLS-terminating proxy where the internal hop is HTTP; it will **not** work on a plain-HTTP site (the browser drops the cookie — keep `CookieStore::new()` for local HTTP dev). Changing its `domain`, `path` or `secure` afterwards makes the store fail closed (save errors → 500). Neither cookie store binds the token to the login session: when the app already has server-side sessions, use `salvo::csrf::SessionStore` (already included by the facade's `csrf` feature; add `session` for the handler) with `SessionHandler` hooped before `Csrf`.

## Rate limiting (feature `rate-limiter`)

```rust
use salvo::rate_limiter::{RateLimiter, RemoteIpIssuer, FixedGuard, MokaStore, BasicQuota};

let limiter = RateLimiter::new(
    FixedGuard::new(),
    MokaStore::new(),
    RemoteIpIssuer,
    BasicQuota::per_second(10),  // 10 requests / second / IP
);

let app = Router::with_path("api").hoop(limiter).push(api_routes());
```

Different `Issuer`s key the limit on different things: `RemoteIpIssuer` (per TCP peer IP), `UserIssuer` (per authenticated user), or a custom one.

**Behind a reverse proxy / load balancer**, `RemoteIpIssuer` sees only the proxy's IP — every client shares one quota. Use **`TrustedProxyIssuer::new([proxy_ip, ...])`**: it honors `X-Forwarded-For` only when the request actually came from one of the listed proxy addresses, walks the chain right-to-left, and (1.1) reads *all* repeated `X-Forwarded-For` fields as one chain and stops at malformed hops, so a client can't mint fresh identities by prepending fake IPs. `ForwardedHeaderIssuer` is deprecated (since 0.94) for exactly that spoofing reason — don't use it.

```rust
use std::net::IpAddr;
use salvo::rate_limiter::TrustedProxyIssuer;

let issuer = TrustedProxyIssuer::new(["10.0.0.1".parse::<IpAddr>().unwrap()]);
```

For production-grade limits across replicas, swap `MokaStore` for a Redis-backed store.

## TLS (features `rustls` / `openssl` / `native-tls`)

```rust
use salvo::conn::rustls::{Keycert, RustlsConfig};

let config = RustlsConfig::new(
    Keycert::new()
        .cert(include_bytes!("../tls/cert.pem").to_vec())
        .key(include_bytes!("../tls/key.pem").to_vec())
);

let acceptor = TcpListener::new("0.0.0.0:443").rustls(config.clone()).bind().await;
Server::new(acceptor).serve(router).await;
```

For HTTP→HTTPS redirect on a separate listener, use `salvo::force_https::ForceHttps::new()` on the HTTP listener or service. Since 1.1 it decides by the **listener's transport** (the connection scheme), not the request's absolute-form URI, so clients can't spoof `https://` to dodge the redirect or cause loops.

**mTLS / TLS metadata (1.1):** `req.tls_info()` exposes the client certificate chain, SNI and ALPN for rustls (and rustls-backed HTTP/3) connections — use it to authorize by client cert. See the official `examples/mtls-rustls` (via Context7 / the repo) before wiring a verifier.

## ACME / Let's Encrypt (feature `acme`; add `quinn` if using `.quinn(...)`)

```rust
let router = Router::new().get(hello);
let mut challenge_router = Router::new();
let listener = TcpListener::new("0.0.0.0:443")
    .acme()
    .cache_path("/var/lib/acme")
    .add_domain("example.com")
    .http01_challenge(&mut challenge_router) // attaches /.well-known/acme-challenge to the :80 router
    .quinn("0.0.0.0:443");                   // optional HTTP/3
// 1.1+: `listener.bind().await` waits until the initial certificate is issued,
// so the :80 challenge server must ALREADY be running — start it first:
let http = TcpListener::new("0.0.0.0:80").bind().await;
let http_task = tokio::spawn(Server::new(http).serve(challenge_router));
let acceptor = listener.bind().await;
Server::new(acceptor).serve(router).await;
http_task.abort();
```

The CA validates HTTP-01 over plain HTTP on **port 80**, so a :80 listener serving the challenge router is mandatory. ⚠️ The pre-1.1 pattern — `http01_challenge(&mut router)` + `listener.join(TcpListener::new("0.0.0.0:80")).bind().await` — no longer works on first issuance: `bind()` now blocks on certificate issuance, but the joined :80 side doesn't serve until `bind()` returns, so validation never completes (the `http01_challenge` source doc says: "Serve `router` on port 80 before awaiting this listener's bind. Initial certificate issuance waits for the CA to read that challenge route."). This mirrors the official 1.1.2 `acme-http01` example. For DNS-01 / TLS-ALPN-01, no port 80 is needed.
