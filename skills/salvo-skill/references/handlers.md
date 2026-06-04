# Handlers

`#[handler]` is a procedural macro that wraps an async function as something implementing `Handler`. The macro inspects the signature and generates the right adapter.

## Allowed parameter shapes

`#[handler]` matches parameters **by type**, not by position. Take only what you need, in any order. Skip what you don't.

The four "context" parameter types the macro recognizes:

```rust
&mut Request    // read incoming
&mut Depot      // request-scoped state
&mut Response   // write outgoing manually
&mut FlowCtrl   // control middleware chain
```

All examples below are valid:

```rust
#[handler] async fn h1() -> &'static str { "ok" }
#[handler] async fn h2(req: &mut Request) -> String { /* ... */ }
#[handler] async fn h3(depot: &mut Depot) -> Json<User> { /* ... */ }
#[handler] async fn h4(res: &mut Response, depot: &mut Depot) { /* ... */ }
#[handler] async fn h5(req: &mut Request, depot: &mut Depot,
                       res: &mut Response, ctrl: &mut FlowCtrl) { /* ... */ }
```

Don't add an `_req: &mut Request` just to "satisfy ordering" — that was true in much older Salvo versions but isn't anymore.

In addition, the macro accepts **typed extractors as parameters**, mixable with the context types above:

```rust
use salvo::oapi::extract::{JsonBody, PathParam, QueryParam}; // NOT in prelude — import explicitly

#[handler]
async fn create_user(json: JsonBody<NewUser>) -> Json<User> { /* ... */ }

#[handler]
async fn show(depot: &mut Depot, id: PathParam<i64>) -> String { /* ... */ }

#[handler]
async fn search(q: QueryParam<String, true>, page: QueryParam<u32, false>) -> Json<Vec<User>> { /* ... */ }
```

See `data-extraction.md` for the full extractor list. These extractor types live in `salvo::oapi::extract` and require the `oapi` feature — they are **not** re-exported by `salvo::prelude`, so paste-ready code must import them. Without `oapi`, use `Request` methods or `Extractible` derive instead.

## Return types — what implements `Writer`

`Writer` is the trait that turns a value into an HTTP response. `#[handler]` accepts any return type that implements `Writer`. Built-in impls:

| Return type | Effect |
|---|---|
| `()` | Empty response (status from `res.status_code` or default 200) |
| `&'static str`, `String`, `Cow<'_, str>` | `text/plain` body |
| `&'static [u8]`, `Vec<u8>`, `Bytes` | `application/octet-stream` body |
| `Text<T>` (`Plain` / `Html` / `Json` / `Xml` / `Js` / `Css` / `Csv` / `Atom` / `Rss` / `Rdf`; `#[non_exhaustive]`) | Sets correct `Content-Type` (note the JS variant is `Js`, not `JavaScript`) |
| `Json<T>` where `T: Serialize` | Serialize as JSON, sets `application/json` |
| `Redirect` | 302 / 301 / etc. with `Location` header |
| `Result<T, E>` where both `T: Writer` and `E: Writer` | Writes whichever side is present |
| `StatusError` | Renders the framework's error page (HTML/JSON depending on `Accept`) |
| `anyhow::Error` (with `anyhow` feature) | Maps to 500 + the error string |
| `Stream<...>`, `SseEvent` (with `sse` feature) | Streamed response |
| Custom `T: Writer` | Whatever the impl does |

Most idiomatic: return `Result<Json<T>, StatusError>` for REST APIs.

## Using the response object directly

When you need fine-grained control:

```rust
#[handler]
async fn custom(res: &mut Response) {
    res.status_code(StatusCode::CREATED);
    res.headers_mut().insert("X-Foo", "bar".parse().unwrap());
    res.render(Json(MyData { /* ... */ }));
}
```

`res.render(x)` calls `x.write(...)`. You can pass anything `Writer`.

For streaming bodies:

```rust
res.stream(my_stream);
```

If you construct byte chunks yourself, add `bytes = "1"` to `Cargo.toml`.

For raw bytes with explicit content type:

```rust
res.add_header("content-type", "application/pdf", true).ok();
res.body(pdf_bytes);
```

## Streaming responses

```rust
use futures_util::stream::iter;
use bytes::Bytes;
use std::convert::Infallible;

#[handler]
async fn stream_handler(res: &mut Response) {
    let s = iter(vec![Ok::<_, Infallible>(Bytes::from("chunk 1\n")), Ok(Bytes::from("chunk 2\n"))]);
    res.stream(s);
}
```

For SSE specifically, see `realtime.md`.

## Custom `Writer` for app-specific types

When your domain wraps responses uniformly (e.g., `ApiResponse<T> { code, msg, data }`), implement `Writer` once and stop calling `Json::new(...)` everywhere:

```rust
use async_trait::async_trait;
use serde::Serialize;
use salvo::prelude::*;

pub struct ApiOk<T: Serialize>(pub T);

#[async_trait]
impl<T: Serialize + Send + 'static> Writer for ApiOk<T> {
    async fn write(self, _req: &mut Request, _depot: &mut Depot, res: &mut Response) {
        res.render(Json(serde_json::json!({ "code": 0, "data": self.0 })));
    }
}

#[handler]
async fn h() -> ApiOk<&'static str> { ApiOk("hi") }
```

For an error counterpart, see `error-handling.md`.

## Common pitfalls

- Prefer `async fn` for handlers, because real handlers usually await I/O. Plain `#[handler] fn ...` still compiles when no await is needed.
- **No `Self` in `#[handler]` on impls**. For struct methods, use `#[craft]` (requires `craft` feature) or wrap the call inside a free function that obtains the receiver from `Depot`.
- **`'static` constraints**: types captured by closures inside handlers must be `Send + 'static`. Wrap non-`Sync` state in `Arc<Mutex<...>>` (or prefer immutable shared state).
