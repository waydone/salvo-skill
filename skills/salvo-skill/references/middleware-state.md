# Middleware (`hoop`) and shared state

## The `Handler` trait — middleware is just a handler

There is **no separate "middleware" trait** in Salvo. Middleware = `impl Handler` attached via `.hoop(...)`. Every middleware sees the same four parameters as a regular handler: `&mut Request`, `&mut Depot`, `&mut Response`, `&mut FlowCtrl`.

A "leaf" handler is one that produces the response. A "middleware" is one that doesn't, or that decorates the request/response on the way through.

## `FlowCtrl` — controlling the chain

```rust
ctrl.skip_rest();   // stop the rest of the chain (response is sent as-is)
ctrl.call_next(req, depot, res).await;  // (rare) explicit invocation of next
```

The default flow is: each middleware in order, then the leaf handler. `skip_rest()` short-circuits — useful for auth middleware that wants to reject before the leaf runs.

## Onion model

```
Router::new()
    .hoop(A)              // outer
    .hoop(B)              // inner
    .get(leaf);

Order on request:  A.before → B.before → leaf → B.after → A.after
```

Each middleware gets to do work on the way **in** (before `ctrl.call_next` / before the leaf) and on the way **out** (after).

In practice you write middleware as a single async function and the macro figures out where the leaf runs:

```rust
#[handler]
async fn timing(req: &mut Request, depot: &mut Depot, res: &mut Response, ctrl: &mut FlowCtrl) {
    let start = std::time::Instant::now();
    ctrl.call_next(req, depot, res).await;       // run rest of chain
    let elapsed = start.elapsed();
    res.headers_mut().insert("x-elapsed-ms",
        elapsed.as_millis().to_string().parse().unwrap());
}
```

If you don't call `ctrl.call_next`, the chain still continues — the macro auto-inserts it after your function returns. To **prevent** the rest from running, call `ctrl.skip_rest()`.

## Custom middleware as a struct

When the middleware needs config:

```rust
use async_trait::async_trait;
use salvo::prelude::*;

pub struct Bearer(pub String);

#[async_trait]
impl Handler for Bearer {
    async fn handle(&self, req: &mut Request, _depot: &mut Depot,
                    res: &mut Response, ctrl: &mut FlowCtrl) {
        let ok = req.header::<String>("authorization")
            .map(|h| h == format!("Bearer {}", self.0))
            .unwrap_or(false);
        if !ok {
            res.status_code(StatusCode::UNAUTHORIZED);
            ctrl.skip_rest();
        }
    }
}

let router = Router::new().hoop(Bearer("secret".into())).get(home);
```

## `Depot` — request-scoped key-value store

Two access patterns:

**By type** (recommended for shared state):

```rust
depot.insert_typed(my_value);             // T-keyed (one per type)
let val = depot.get_typed::<MyType>(); // -> Result<&MyType, _>
```

**By string key** (per-request data):

```rust
depot.insert("user_id", 42_u64);
let id = depot.get::<u64>("user_id");  // Result<&u64, _>
```

## `affix_state` — DI for app-wide state

The `affix-state` Cargo feature provides a builder to inject typed state once and have every downstream handler see it:

```rust
use salvo::prelude::*;
use salvo::affix_state;
use std::sync::Arc;

#[derive(Clone)]
struct AppConfig { db_url: String }

let cfg = AppConfig { db_url: "postgres://...".into() };
let pool = Arc::new(/* sqlx::PgPool::connect(...).await? */);

let router = Router::new()
    .hoop(
        affix_state::inject(cfg)
            .inject(pool.clone())
            .insert("build_sha", env!("CARGO_PKG_VERSION"))
    )
    .get(handler);

#[handler]
async fn handler(depot: &mut Depot) -> Result<String, StatusError> {
    let cfg = depot.get_typed::<AppConfig>()
        .map_err(|_| StatusError::internal_server_error())?;
    let _build_sha = depot.get::<&str>("build_sha");
    Ok(cfg.db_url.clone())
}
```

Rules of thumb:
- Inject **clonable, cheap-to-clone things** like `Arc<PgPool>`, `Arc<RwLock<Cache>>`, config structs.
- Inject **once at the root router** so all routes see it.
- Wrap mutable state in `Arc<Mutex<...>>` or `Arc<RwLock<...>>`. Don't try to inject `&mut T`.

## Conditional middleware (`hoop_when`)

```rust
let router = Router::new()
    .hoop_when(Logger::new(), |req, _depot| !req.uri().path().starts_with("/health"))
    .get(home)
    .push(Router::with_path("health").get(health));
```

The closure runs **per request**. Keep it cheap.

## Pattern: scoped state

When only one branch needs a piece of state (e.g. a per-tenant DB pool), inject inside that branch:

```rust
let app = Router::new()
    .hoop(affix_state::inject(global_config))
    .push(
        Router::with_path("tenant-a")
            .hoop(affix_state::inject(tenant_a_pool))
            .push(tenant_routes())
    )
    .push(
        Router::with_path("tenant-b")
            .hoop(affix_state::inject(tenant_b_pool))
            .push(tenant_routes())
    );
```

Inside `tenant_routes`, `depot.get_typed::<PgPool>()` returns whichever tenant pool was injected at the parent.
