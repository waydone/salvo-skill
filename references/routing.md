# Routing

Salvo's router is a tree. Each node has:

- **path filter** (optional) — a string pattern matched against the request URL
- **method/host/scheme/header filters** (optional)
- **middleware** (`hoop`)
- **handler** (`get` / `post` / ... / `goal` / `handle`)
- **children** (`push`)

A request walks the tree top-down. At each node, all filters must match. The first leaf whose filters all pass is the handler that runs.

## Path syntax (0.76+)

```rust
Router::with_path("articles/{id}")              // single segment, captured as "id"
Router::with_path("articles/{id|\\d+}")         // regex constraint (use raw string)
Router::with_path("files/{**rest}")             // greedy multi-segment, captured as "rest"
Router::with_path("files/{*name}")              // single-segment but optional name
Router::with_path(r"users/{id|\d+}/posts")      // mixed regex + literals
Router::with_path("static/<**path>")            // ❌ OLD SYNTAX — gone since 0.76
```

Read parameters in the handler:

```rust
#[handler]
async fn show(req: &mut Request) -> String {
    let id: i64 = req.param("id").unwrap_or_default();
    let rest: String = req.param("rest").unwrap_or_default();
    format!("id={id}, rest={rest}")
}
```

Or with the typed extractor:

```rust
#[handler]
async fn show(id: PathParam<i64>) -> String {
    format!("id={}", id.into_inner())
}
```

## Method filters

Shortcuts on `Router`:

```rust
Router::with_path("users").get(list).post(create);
Router::with_path("users/{id}").get(show).put(update).patch(patch).delete(remove);
```

If you need a method without a shortcut (e.g. `LINK`), use `filter`:

```rust
use salvo::routing::filters;
Router::with_path("links").filter(filters::method(Method::LINK)).handle(link_handler);
```

## `get` vs `goal` vs `handle`

| Method | When to use |
|---|---|
| `.get(h)` / `.post(h)` / etc | The handler runs **only if** the method matches AND no further sub-path. Most cases. |
| `.handle(h)` | Like `.get` but matches **any** method. Useful when the same function handles multiple methods internally. |
| `.goal(h)` | Matches **anything** at this node — any method, any sub-path. Use for catch-alls and WebSocket upgrade paths (`Router::with_path("ws").goal(connect)`). |

`.goal` is a footgun if used by accident — it'll swallow paths you didn't expect. When in doubt, use `.get` / `.post` / etc.

## Nesting (`push`)

```rust
Router::new()
    .push(
        Router::with_path("api/v1")
            .hoop(Logger::new())
            .push(Router::with_path("users").get(list_users).post(create_user))
            .push(Router::with_path("users/{id}").get(get_user).delete(del_user))
            .push(Router::with_path("posts").get(list_posts))
    )
    .push(
        Router::with_path("admin")
            .hoop(jwt_auth())
            .push(Router::with_path("stats").get(admin_stats))
    );
```

A child inherits its parents' filters. The middleware order is parent-to-child. Each child's `path` is **relative** to the parent's path — so the child of `Router::with_path("api/v1")` with path `users` matches `/api/v1/users`.

## Custom filters

`Router::filter(...)` accepts anything that implements `Filter`. The most common builders are in `salvo::routing::filters`:

```rust
use salvo::routing::filters;

Router::new()
    .filter(filters::host("api.example.com"))
    .filter(filters::scheme(salvo::http::uri::Scheme::HTTPS))   // scheme() takes a Scheme, NOT &str
    .filter(filters::header("x-tenant"))
    .filter_fn(|req, _path_state| req.header::<String>("x-version") == Some("v2".into())); // 2nd arg is &mut PathState, not Depot
```

Combine with `.and()` / `.or()`:

```rust
Router::new().filter(filters::path("hello").and(filters::get()));
```

## Common patterns

**API versioning:**

```rust
fn v1_router() -> Router {
    Router::with_path("v1")
        .push(Router::with_path("users").get(v1::list).post(v1::create))
}

fn v2_router() -> Router {
    Router::with_path("v2")
        .push(Router::with_path("users").get(v2::list).post(v2::create))
}

let api = Router::with_path("api").push(v1_router()).push(v2_router());
```

**Auth boundary:**

```rust
let public = Router::with_path("public").get(public_index);
let private = Router::with_path("private").hoop(jwt_auth()).get(private_index);
let app = Router::new().push(public).push(private);
```

**Modular composition:** put each feature's routes in its own module, expose `pub fn router() -> Router`, then merge in `main`. Once you have more than ~5 routes, this is the right shape.
