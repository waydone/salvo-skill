# Static files & uploads

## Serving a directory (feature `serve-static`)

```rust
use salvo::prelude::*;
use salvo::serve_static::StaticDir;

let app = Router::new()
    .push(
        Router::with_path("static/{**path}")
            .get(StaticDir::new(["./public", "./assets"])
                .defaults("index.html")
                .auto_list(false))
    );
```

`StaticDir::new(...)` accepts one or more roots — they're tried in order. Useful when you have a fallback (e.g. user uploads in `./uploads`, static assets in `./public`).

Builder options:
- `.defaults("index.html")` — file to serve for directory requests
- `.auto_list(true)` — list directory contents when no default file
- `.include_dot_files(true)` — serve `.env`-like files (default: false, for safety)
- `.fallback("index.html")` — serve this for any unmatched path (great for SPAs)
- `.chunk_size(bytes)` — chunk size when streaming larger files (default 1 MB)

## SPA hosting pattern

```rust
let api = Router::with_path("api").push(/* api routes */);

let spa = Router::with_path("{**path}").get(
    StaticDir::new(["./web/dist"])
        .defaults("index.html")
        .fallback("index.html")  // any unknown path → SPA's index.html
);

let app = Router::new().push(api).push(spa);
```

Order matters — put `api` before `spa` so API routes match first.

## Embedded assets (compiled-in)

For single-binary deployments, use `rust-embed`:

```toml
[dependencies]
rust-embed = "8"
salvo = { version = "0.93.0", features = ["serve-static"] }
mime_guess = "2"
```

```rust
use rust_embed::Embed;
use salvo::prelude::*;

#[derive(Embed)]
#[folder = "web/dist/"]
struct Assets;

#[handler]
async fn serve_asset(req: &mut Request, res: &mut Response) {
    let path = req.param::<String>("path").unwrap_or_else(|| "index.html".into());
    let path = if path.is_empty() { "index.html".into() } else { path };

    match Assets::get(&path).or_else(|| Assets::get("index.html")) {
        Some(file) => {
            let mime = mime_guess::from_path(&path).first_or_octet_stream();
            res.headers_mut().insert("content-type", mime.as_ref().parse().unwrap());
            res.body(file.data.into_owned());
        }
        None => res.status_code(StatusCode::NOT_FOUND),
    }
}

let app = Router::with_path("{**path}").get(serve_asset);
```

## Single-file handler

```rust
use salvo::serve_static::StaticFile;

let app = Router::with_path("favicon.ico").get(StaticFile::new("./assets/favicon.ico"));
```

## Streamed file download

For files larger than ~100 MB, stream rather than buffer:

```rust
use tokio::fs::File;
use tokio_util::io::ReaderStream;

#[handler]
async fn download(res: &mut Response) -> Result<(), StatusError> {
    let file = File::open("/srv/large.zip").await
        .map_err(|_| StatusError::not_found())?;
    res.headers_mut().insert("content-type", "application/zip".parse().unwrap());
    res.headers_mut().insert("content-disposition",
        r#"attachment; filename="archive.zip""#.parse().unwrap());
    res.stream(ReaderStream::new(file));
    Ok(())
}
```

## Uploads (multipart)

```rust
use salvo::prelude::*;

#[handler]
async fn upload(req: &mut Request) -> Result<Json<Vec<String>>, StatusError> {
    let mut saved = Vec::new();

    // single file
    if let Some(file) = req.file("avatar").await {
        let dest = std::path::PathBuf::from("./uploads")
            .join(file.name().unwrap_or("upload.bin"));
        std::fs::create_dir_all("./uploads").ok();
        std::fs::copy(file.path(), &dest)
            .map_err(|e| StatusError::internal_server_error().detail(e.to_string()))?;
        saved.push(dest.display().to_string());
    }

    // multiple files under same field name
    for file in req.files("attachments").await.into_iter().flatten() {
        let dest = std::path::PathBuf::from("./uploads")
            .join(file.name().unwrap_or("upload.bin"));
        std::fs::copy(file.path(), &dest).ok();
        saved.push(dest.display().to_string());
    }

    Ok(Json(saved))
}
```

`req.file("name")` returns `Option<FilePart>`. `req.files("name")` returns `Option<Vec<FilePart>>`.

`FilePart` exposes `.path()` (temp path), `.name()` (original filename), `.content_type()`, `.headers()`, `.size()`. The temp file is deleted when the **`FilePart`** drops (not when `Request` drops), so persist it before the `FilePart` goes out of scope — or call `file.do_not_delete_on_drop()` to manage cleanup yourself.

## Upload size limit

Salvo applies a **64KB request-body limit by default** (a DoS guard). That's far too small for real uploads, so the task is usually to **raise** the cap, not add one. Don't hand-roll it — Salvo ships size limiting under the `size-limiter` feature:

```rust
use salvo::prelude::*;   // requires feature "size-limiter" — NOT included in "full", add it explicitly

let app = Router::new()
    .hoop(max_size(20 * 1024 * 1024))   // raise the cap to 20 MB
    .push(/* routes */);
```

`max_size(bytes)` is a helper fn returning a `MaxSize` middleware that checks the declared `content-length`. When you can't trust the declared length (chunked bodies), use the **`SecureMaxSize` struct** — there is no `secure_max_size()` helper fn; construct it directly: `.hoop(salvo::http::request::SecureMaxSize::new(20 * 1024 * 1024))`. It enforces the limit against the bytes actually read.
