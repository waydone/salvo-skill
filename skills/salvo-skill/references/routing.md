# Routing

Salvo's router is a tree. Each node has:

- **path filter** (optional) — a string pattern matched against the request URL
- **method/host/scheme/header filters** (optional)
- **middleware** (`hoop`)
- **handler** (`get` / `post` / ... / `query` / `goal`)
- **children** (`push`)

A request walks the tree top-down. At each node, all filters must match. The first leaf whose filters all pass is the handler that runs.

## Path syntax (0.76+)

```rust
Router::with_path("articles/{id}")              // single segment, captured as "id"
Router::with_path("articles/{id|\\d+}")         // regex constraint (use raw string)
Router::with_path("files/{**rest}")             // multi-segment wildcard, matches zero+ segments (incl. empty)
Router::with_path("files/{*+rest}")             // multi-segment wildcard, requires at least one non-empty segment
Router::with_path("files/{*?rest}")             // single-segment wildcard, may be empty
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

If you need a method without a shortcut (e.g. `LINK`), use `filter_fn` and set the handler with `goal`:

```rust
Router::with_path("links")
    .filter_fn(|req, _path_state| req.method().as_str() == "LINK")
    .goal(link_handler);
```

## `get` vs `goal`

| Method | When to use |
|---|---|
| `.get(h)` / `.post(h)` / etc | The handler runs **only if** the method matches AND no further sub-path. Most cases. |
| `.goal(h)` | Matches **anything** at this node — any method, any sub-path. Use for catch-alls and WebSocket upgrade paths (`Router::with_path("ws").goal(connect)`). |

There is no `Router::handle(...)` (still true in 1.1.2). `.goal` is a footgun if used by accident — it'll swallow paths you didn't expect. When in doubt, use `.get` / `.post` / etc.

There is a first-class shortcut for the HTTP **`QUERY`** method (a safe, body-carrying read used by some search APIs): `Router::with_path("search").query(search_handler)` (and the matching `filters::query()`). Use it directly — you don't need the `filter_fn` custom-method workaround for QUERY. OpenAPI: since 1.0 a `QUERY` route **is** emitted into the generated doc, but **only** when the document declares OpenAPI 3.2 — in the default 3.1 doc it is skipped with a `tracing::warn!` (see `references/openapi.md`).

**405 / `Allow` (1.1):** when a path matches but the method doesn't, Salvo answers **405** with an `Allow` header computed from the built-in method filters on that path. If only opaque filters (e.g. a `filter_fn` method check like the `LINK` example above) guard the path, Salvo can't infer the methods and falls back to **404** — add your own 405 + `Allow` in middleware if clients need it.

## Path normalization and rejected paths (1.1.2)

Before any route is matched, Salvo normalizes dot segments (`.` / `..`, including percent-encoded forms) and **rejects ambiguous paths with `400 Bad Request`**: an encoded separator (`%2F`, `%5C`), a literal backslash, or a NUL byte anywhere in the path (on Windows also colon-bearing segments and reserved device names). The point is that a later decode — in a static-file handler, a proxy, your own code — can no longer move a request outside the scope the router authorized.

What it means for route design:

- `GET /files/a%2Fb` no longer reaches a `files/{name}` handler with `name = "a/b"` — it is a 400 (verified with `TestClient`). To accept nested names, capture the tail with `files/{**rest}` and let the client send real slashes, or pass the value in the query string / body.
- IDs that may contain `/` (object-store keys, some base64) must be encoded as base64url or sent outside the path.
- The 400 is produced before your hoops run, so it does not show up in handler-level metrics or auth logs.

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
use salvo::routing::Filter;

Router::new()
    .filter(filters::host("api.example.com"))
    .filter(filters::scheme(salvo::http::uri::Scheme::HTTPS))   // scheme() takes a Scheme, NOT &str
    .filter_fn(|req, _path_state| req.header::<String>("x-tenant").is_some())
    .filter_fn(|req, _path_state| req.header::<String>("x-version") == Some("v2".into())); // 2nd arg is &mut PathState, not Depot
```

Combine with `.and()` / `.or()`:

```rust
use salvo::routing::filters::{self, Filter}; // `and` / `or` are methods of the `Filter` trait

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
