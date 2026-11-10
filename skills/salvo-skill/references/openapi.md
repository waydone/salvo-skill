# OpenAPI (feature `oapi`)

The `salvo-oapi` crate, exposed via the `oapi` feature, generates OpenAPI 3.1 documentation by inspecting `#[endpoint]`-annotated handlers, `JsonBody<T>` / `PathParam<T>` / etc. extractors, and `ToSchema`-derived types.

## Minimal end-to-end example

```rust
use salvo::prelude::*;
use salvo::oapi::extract::{JsonBody, PathParam}; // same types as salvo::extract::{...} (core since 0.96)
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, ToSchema)]
struct User {
    id: i64,
    name: String,
}

#[derive(Deserialize, ToSchema)]
struct NewUser {
    name: String,
}

#[endpoint(
    tags("users"),
    responses(
        (status_code = 200, body = User, description = "user found"),
        (status_code = 404, description = "not found"),
    )
)]
async fn show(id: PathParam<i64>) -> Result<Json<User>, StatusError> {
    Ok(Json(User { id: id.into_inner(), name: "Ada".into() }))
}

#[endpoint(tags("users"), status_codes(201))]
async fn create(body: JsonBody<NewUser>) -> Json<User> {
    Json(User { id: 1, name: body.into_inner().name })
}

#[tokio::main]
async fn main() {
    let router = Router::new()
        .push(Router::with_path("users/{id}").get(show))
        .push(Router::with_path("users").post(create));

    let doc = OpenApi::new("My API", "1.0.0").merge_router(&router);

    let app = Router::new()
        .push(doc.into_router("/api-doc/openapi.json"))
        .push(SwaggerUi::new("/api-doc/openapi.json").into_router("/swagger"))
        .push(Scalar::new("/api-doc/openapi.json").into_router("/scalar"))
        .push(router);

    let acceptor = TcpListener::new("0.0.0.0:5800").bind().await;
    Server::new(acceptor).serve(app).await;
}
```

Visit:
- `/api-doc/openapi.json` — the spec
- `/swagger` — Swagger UI
- `/scalar` — Scalar UI (modern alternative)

`RapiDoc` and `ReDoc` are also available with the same `.into_router(...)` pattern.

### OpenAPI version (3.1 default, 3.2 opt-in)

`OpenApi::new(...)` still emits **OpenAPI 3.1** by default. **3.2** is opt-in per document with `.openapi_version(...)`:

```rust
use salvo::oapi::{OpenApi, OpenApiVersion};
let doc = OpenApi::new("My API", "1.0.0")
    .openapi_version(OpenApiVersion::Version3_2)   // 3.1 stays the default
    .merge_router(&router);
```

The enum variants are `OpenApiVersion::Version3_1` (default) and `Version3_2`. What 3.2 buys you **since 1.0** (verified in salvo-oapi 1.1.1 source, `openapi.rs` route discovery):

- **`Router::query(...)` routes are emitted** as `query` operations (`PathItemType::Query`), and routes on **custom HTTP methods** go into `additionalOperations`. In a document that declares **3.1** those routes are **skipped** — `merge_router` logs a `tracing::warn!` ("HTTP method has no OpenAPI 3.1 representation… call `OpenApi::openapi_version` with `OpenApiVersion::Version3_2`"). So if a `QUERY` endpoint is missing from your doc, that warning is why.
- The 3.2 object model: Tag `summary`/`parent`/`kind`, Server `name`, Parameter `in: querystring` (`ParameterIn::QueryString`) and `style: cookie`, Media Type `itemSchema`/`prefixEncoding`/`itemEncoding`, Response `summary`, Components `mediaTypes`, the `deviceAuthorization` OAuth flow, `$self`.
- UI caveat (from Salvo's 1.0 notes): of the bundled UIs only **Swagger UI** renders `query` operations; Scalar / RapiDoc / ReDoc load a 3.2 document but silently omit them.

Switch to 3.2 when you actually have `QUERY`/custom-method routes to document or a consumer requires a `3.2.x` document; otherwise 3.1 is the safer default for tooling.

## `#[endpoint]` vs `#[handler]`

Both work as Salvo handlers. `#[endpoint]` does everything `#[handler]` does, plus registers the operation with the OpenAPI doc. **In an `oapi`-enabled crate, prefer `#[endpoint]` for any HTTP-facing handler** — purely-internal handlers (custom middleware) can stay on `#[handler]`.

Doc comments on the function are used for `summary` (first line) and `description` (rest).

## `#[endpoint]` attributes

```rust
#[endpoint(
    operation_id = "show_user",
    tags("users", "public"),
    parameters(
        ("limit" = u32, Query, description = "page size", example = 20)
    ),
    request_body(content = NewUser, description = "user payload"),
    responses(
        (status_code = 200, body = User, description = "ok",
         content_type = ["application/json"], example = json!({"id":1,"name":"Ada"})),
        (status_code = 400, description = "bad input"),
        (status_code = 500, description = "server error"),
    ),
    security(["bearer" = []]),
)]
```

Most of these you can omit — the macro infers them from the function signature. Override only what you need.

## `ToSchema` for response/request types

```rust
#[derive(Serialize, Deserialize, ToSchema)]
#[salvo(schema(example = json!({"id": 1, "name": "Ada"})))]
struct User {
    /// Stable user identifier.
    id: i64,
    /// Display name.
    name: String,
}
```

Borrowed types don't work in `ToSchema` — use `String` / `Vec<u8>` in your DTOs even if the extracted type is `&str` / `&[u8]`.

For enums, the macro generates a `oneOf` discriminator if you tag it — see Salvo's docs for `#[salvo(schema(...))]` attributes.

## `ToParameters` for query / path / header param structs

When you want a single struct to represent all query params:

```rust
#[derive(Deserialize, ToParameters)]
#[salvo(parameters(default_parameter_in = Query))]
struct ListParams {
    /// Maximum results to return.
    #[salvo(parameter(example = 20))]
    limit: Option<u32>,
    /// Offset for pagination.
    offset: Option<u32>,
}

#[endpoint]
async fn list(params: ListParams) -> Json<Vec<User>> { /* ... */ }
```

This auto-registers all fields as query parameters.

## Security schemes

```rust
use salvo::oapi::{OpenApi, SecurityScheme, security::{Http, HttpAuthScheme}};

let doc = OpenApi::new("My API", "1.0.0")
    .add_security_scheme("bearer",
        SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer).bearer_format("JWT")))
    .merge_router(&router);
```

Then on protected endpoints add `security(["bearer" = []])`.

## Common pitfalls

1. **Forgetting `merge_router(&router)`**: without it, `OpenApi` has no operations.
2. **`merge_router` order**: it must come **after** all routes are pushed to the router. Build the API router first, then merge.
3. **Mounting the doc router under the main router**: a footgun if your main router has a path prefix, since the doc URL gets prefixed too. Either keep the doc router at the top level, or include the prefix in the SwaggerUi/Scalar URL.
4. **`#[endpoint]` outside an `oapi`-enabled crate**: compile error. Either enable the feature or fall back to `#[handler]`.
5. **Missing extractor imports**: `JsonBody<T>` / `PathParam<T>` / `QueryParam<T, _>` are **not** in `salvo::prelude`. Import them from `salvo::extract` (core, since 0.96) or `salvo::oapi::extract` (re-export of the same types) — both register schemas under `#[endpoint]`.
6. **Building `Parameters` by hand (1.1 break)**: `Parameters.0` is now `Vec<RefOr<Parameter>>` so lists can hold `$ref`s. `Parameters(vec)` with a `Vec<Parameter>` no longer compiles — write `Parameters::from(vec)`; when reading the list, match `RefOr` or use `.iter_inline()`. Inline builders (`.parameter(...)`) still take a plain `Parameter`. Similarly `Header::schema` is now `Option<RefOr<Schema>>` (1.0), and 1.0 added enum variants (`PathItemType::Query`, `ParameterIn::QueryString`, `ParameterStyle::Cookie`, `Flow::DeviceAuthorization`) — exhaustive `match`es over these need a new arm.
7. **`QueryString` in `ToParameters` (1.1)**: deriving `ToParameters` with a `QueryString` location is now rejected (it produced an invalid schema). Use `Query` for named values; build a whole-query `content` parameter manually if you really need `in: querystring`.
8. **Long-lived borrows in DTOs**: don't use `&'a str` in `ToSchema`-derived types — `serde_json` deserialization plus schema generation needs owned types.

## When the user wants OpenAPI for an existing app

Likely changes:
- Add `oapi` to features.
- Replace `#[handler]` → `#[endpoint]` on HTTP-facing handlers.
- Add `ToSchema` to all request/response types.
- Replace `req.parse_*` / imperative extraction with `JsonBody<T>` / `PathParam<T>` / `QueryParam<T, _>` / `ToParameters` structs so the schemas auto-register.
- Add the doc router and SwaggerUi/Scalar routes in `main`.
