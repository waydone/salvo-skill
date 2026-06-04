# Realtime: WebSocket and SSE

## WebSocket (feature `websocket`)

The WebSocket handler upgrades a regular request and gives you a typed channel.

```rust
use salvo::prelude::*;
use salvo::websocket::{Message, WebSocketUpgrade};

#[handler]
async fn ws_handler(req: &mut Request, res: &mut Response) -> Result<(), StatusError> {
    WebSocketUpgrade::new()
        .upgrade(req, res, |mut ws| async move {
            while let Some(msg) = ws.recv().await {
                let Ok(msg) = msg else { return; };  // client gone
                if msg.is_text() {
                    let Ok(text) = msg.as_str() else { continue; };
                    if ws.send(Message::text(format!("echo: {text}"))).await.is_err() {
                        return;
                    }
                } else if msg.is_binary() {
                    if ws.send(msg).await.is_err() { return; }
                } else if msg.is_close() {
                    return;
                }
                // is_ping / is_pong: handled automatically by the runtime
            }
        })
        .await
}

let app = Router::new().push(Router::with_path("ws").goal(ws_handler));
```

Use `.goal(ws_handler)` (not `.get(...)`) on the WebSocket route. The upgrade isn't a normal GET; `.goal` lets the handler match regardless.

**Important: `Message` is an opaque struct (still true in 0.93), not an enum.** Older docs and tutorials show pattern matching like `match msg { Message::Text(t) => ... }` — that compiles on Salvo ≤0.65 only. The current API is method-based:

| Inspect | Construct |
|---|---|
| `msg.is_text()` / `is_binary()` / `is_close()` / `is_ping()` / `is_pong()` | `Message::text(s)` / `Message::binary(v)` |
| `msg.as_str() -> Result<&str, _>` | `Message::ping(v)` / `Message::pong(v)` |
| `msg.as_bytes() -> &[u8]` | `Message::close()` / `Message::close_with(code, reason)` |

Don't try to destructure `Message` with `if let Message::Text(t)` — it won't compile.

## Pre-upgrade auth / config

Inspect the request before upgrading:

```rust
#[handler]
async fn ws_handler(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), StatusError> {
    let user = depot.jwt_auth_data::<Claims>()
        .ok_or_else(StatusError::unauthorized)?
        .claims.sub.clone();

    WebSocketUpgrade::new()
        .upgrade(req, res, move |mut ws| async move {
            tracing::info!(user, "ws connected");
            // ...
        })
        .await
}
```

You can also place a JWT middleware at the parent router and the upgrade still works.

## Broadcast / pub-sub

For a chat-style fan-out, share a `tokio::sync::broadcast::Sender` via `affix_state`:

```rust
use tokio::sync::broadcast;

#[derive(Clone)]
struct ChatState { tx: broadcast::Sender<String> }

#[handler]
async fn ws_chat(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), StatusError> {
    let state = depot.obtain::<ChatState>()
        .map_err(|_| StatusError::internal_server_error())?
        .clone();

    WebSocketUpgrade::new()
        .upgrade(req, res, move |mut ws| async move {
            let mut rx = state.tx.subscribe();
            loop {
                tokio::select! {
                    incoming = ws.recv() => {
                        let Some(Ok(msg)) = incoming else { return; };
                        if msg.is_close() { return; }
                        if msg.is_text() {
                            if let Ok(text) = msg.as_str() {
                                let _ = state.tx.send(text.to_owned());
                            }
                        }
                    }
                    outgoing = rx.recv() => {
                        let Ok(text) = outgoing else { continue; };
                        if ws.send(Message::text(text)).await.is_err() { return; }
                    }
                }
            }
        })
        .await
}

#[tokio::main]
async fn main() {
    let (tx, _) = broadcast::channel::<String>(256);
    let state = ChatState { tx };
    let router = Router::new()
        .hoop(salvo::affix_state::inject(state))
        .push(Router::with_path("ws").goal(ws_chat));
    /* ... */
}
```

For per-room broadcast, key a `HashMap<RoomId, broadcast::Sender>` behind an `Arc<RwLock<...>>`.

## SSE (feature `sse`)

Server-sent events stream a one-way feed of named events. Simpler than WebSocket when you only need server→client push.

```rust
use salvo::prelude::*;
use salvo::sse::{self, SseEvent, SseKeepAlive};
use std::convert::Infallible;
use std::time::Duration;
use futures_util::stream::{self, Stream, StreamExt};

fn ticks() -> impl Stream<Item = Result<SseEvent, Infallible>> {
    let interval = tokio::time::interval(Duration::from_secs(1));
    tokio_stream::wrappers::IntervalStream::new(interval)
        .enumerate()
        .map(|(i, _)| {
            Ok(SseEvent::default()
                .name("tick")
                .id(i.to_string())
                .text(format!("tick {i}")))
        })
}

#[handler]
async fn stream_handler(res: &mut Response) {
    SseKeepAlive::new(ticks())
        .max_interval(Duration::from_secs(15))
        .stream(res);
}

let app = Router::new().push(Router::with_path("events").get(stream_handler));
```

`SseEvent` builder methods:
- `.name(...)` — event name (`event:` field)
- `.text(...)` — event data (`data:` field)
- `.id(...)` — event id (`id:` field, used for reconnection)
- `.retry(Duration)` — reconnection delay hint

Wrap the stream in `SseKeepAlive` so idle connections don't get killed by proxies.

## When to choose WebSocket vs SSE

| Use SSE if… | Use WebSocket if… |
|---|---|
| Server → client only (live feed, notifications, model output streaming) | Bi-directional (chat, collaborative editing, games) |
| Plain HTTP works (proxies, browsers, retries via EventSource) | Need binary frames, low latency, or custom protocols |
| You want auto-reconnect | You're managing reconnection yourself |

If you only need streaming responses from an LLM-style API, SSE is almost always the right choice.
