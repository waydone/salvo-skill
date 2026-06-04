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

For RSA / ECDSA decoders use `RsaDecoder`, `EcdsaDecoder`. For dynamic keys (e.g. JWKS rotation), implement `JwtAuthDecoder` yourself.

## Basic auth (feature `basic-auth`)

```rust
use salvo::prelude::*;
use salvo::basic_auth::{BasicAuth, BasicAuthValidator};

struct Validator;

// No #[async_trait] — BasicAuthValidator uses native RPITIT (async fn in trait) in 0.93; adding it fails E0195.
impl BasicAuthValidator for Validator {
    async fn validate(&self, username: &str, password: &str, _depot: &mut Depot) -> bool {
        username == "admin" && password == "secret"
    }
}

let auth = BasicAuth::new(Validator);
let app = Router::new().hoop(auth).get(home);
```

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
    s.insert("user_id", 42)?;
    Ok("logged in")
}
```

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

The default skipper already ignores safe methods and validates `POST` / `PATCH` / `DELETE` / `PUT`. In templates, read `depot.csrf_token()` (via `CsrfDepotExt`) and put it in a hidden form field named to match the finder.

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

Different `Issuer`s key the limit on different things: `RemoteIpIssuer` (per IP), `UserIssuer` (per authenticated user), or a custom one.

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

For HTTP→HTTPS redirect on a separate listener, use `salvo::force_https::ForceHttps::new()` on the HTTP listener or service.

## ACME / Let's Encrypt (feature `acme`; add `quinn` if using `.quinn(...)`)

```rust
let acceptor = TcpListener::new("0.0.0.0:443")
    .acme()
    .cache_path("/var/lib/acme")
    .add_domain("example.com")
    .http01_challenge(&mut router)
    .quinn("0.0.0.0:443")  // optional HTTP/3
    .bind()
    .await;
```

Salvo handles the ACME challenge by attaching a route to your router. Make sure port 80 (HTTP-01) or DNS (DNS-01) is reachable.
