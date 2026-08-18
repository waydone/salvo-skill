# salvo-skill

A Claude Code / agent skill for the **[Salvo](https://salvo.rs) Rust web framework** — target **0.95.2**. Install as a plugin (see below) or by symlinking into `~/.claude/skills/`.

Encodes Salvo's 0.95.2 API surface (project skeleton, routing, `#[handler]`/`#[craft]`, `Writer`/`Scribe`, extractors, `JwtAuth`, middleware/`Depot`, OpenAPI, WebSocket/SSE, static files & tus resumable uploads, response cache, OpenTelemetry, multi-listener/Unix socket, error handling, testing, ops) as a lean **hub-and-spoke** skill: a compact `SKILL.md` + on-demand `references/*.md` (×13), plus `evals/`.

## Discipline baked in

- **Verify with Context7 before non-trivial code** — Salvo's API shifts across minor versions; the skill mandates a Context7 doc check for anything beyond a trivial `Router`.
- **edition 2024** skeleton (edition needs Rust 1.85, but Salvo 0.95.2's MSRV is **Rust 1.94**); for existing projects, match what's there.
- `0.95.2` (from `0.93.0`) is *almost* source-compatible — most changes are `#[deprecated]` warnings, the one hard break being the string-keyed `Depot::remove` signature (`remove::<T>("k")` → `remove(key) -> Option<Box<dyn Any>>`). `0.94.0` renamed the type-keyed `Depot` accessors (`obtain`→`get_typed`, `inject`→`insert_typed`, `scrape`→`remove_typed` — old names deprecated-but-compiling), split the JWT crypto backend (`jwt-auth` = jsonwebtoken `aws_lc_rs` default / `jwt-auth-ring` = RustCrypto), renamed `stop_forcible`→`stop_forceful` and `Response::stuff`→`render_with_status`, and raised MSRV to Rust 1.94. `SKILL.md` opens with a "what changed 0.93 → 0.95.2" section.

## Verification

Every load-bearing API across `SKILL.md` + all `references/*.md` was checked two ways (2026-06): (1) Context7 doc cross-verification of all 14 files, and (2) a real `cargo check` against salvo 0.93.0 that caught and fixed 11 compile-level inaccuracies (extractor imports not in prelude, `parse_queries` is sync, `filters::scheme(Scheme::HTTPS)`, CORS `allow_headers(vec!…)`, `AllowOrigin::any()`, `SecureMaxSize::new`, `default_parameter_in`, …) — then re-compiled to 0 errors.

The v0.1.3 round (2026-06-10) added three more layers: a **content audit** against crate sources (fixed: `Timeout` default is **503** not 408; removed the nonexistent `force-host` feature and `parse_msgpack`; ACME HTTP-01 needs `.join(TcpListener :80)`), **`cargo run` verification** of every newly added snippet (craft / cache / otel / tus / wildcards / StaticDir builder), and an **adversarial review** — 25 claims attacked against local 0.93.0 crate sources, 0 refuted, 5 precision-fixed (e.g. SSE streams must be `TryStream<Ok = SseEvent>`; `RequestIssuer::default()` keys on all five URL parts). Bonus find: salvo-otel's own doc example for `Tracing::new()` is wrong upstream — this skill has the correct signature.

The v0.1.4 round (2026-08-17) is the **0.93 → 0.95.2 bump**. Every change was verified against the actual 0.95.2 crate sources pulled into the local registry (`salvo_core`, `salvo-jwt-auth`, `salvo_extra`, `salvo-oapi`, `salvo-otel`) — not from release notes alone: the `Depot`/`Server`/`Response`/`StatusError` renames were confirmed to be `#[deprecated(since = "0.94.0")]` aliases (still compile); the JWT crypto split (`jwt-auth` = jsonwebtoken `aws_lc_rs`, `jwt-auth-ring` = `rust_crypto`) was read from both Cargo.tomls; `salvo-otel` was confirmed to now need `opentelemetry = "0.32"` (was 0.31); and `OpenApiVersion::Version3_2` (not `V3_2`) was pinned from source. Then a real `cargo check` of the full stack **plus** a snippet exercising every changed API (`get_typed`/`insert_typed`/`remove_typed`, `render_with_status`, `max_connections`, `stop_forceful`, `affix_state::inject`, `openapi_version(Version3_2)`) compiled to **0 errors** on Rust 1.97.

## Changelog

- **0.1.4** (2026-08-17) — bumped target 0.93.0 → **0.95.2** (source-verified). New "what changed 0.93 → 0.95.2" section in `SKILL.md`; `Depot` accessors → `get_typed`/`insert_typed`/`remove_typed` throughout; string-keyed `Depot::remove` signature-break + `delete` deprecation called out; JWT `aws_lc_rs` vs `jwt-auth-ring` (RustCrypto) guidance incl. the `full`-still-pulls-AWS-LC caveat; `stop_forcible`→`stop_forceful` + `max_connections`/`fuse_config`; `render_with_status`; otel `0.31`→`0.32`; MSRV 1.94; real `Router::query()` shortcut; OpenAPI 3.2 opt-in (scoped to `$self` + doc version — QUERY is **not** emitted). A two-model adversarial review (grok-4.6 + deepseek-v4-flash, each cross-checking every claim against local 0.95.2 crate source) caught and fixed: a bulk-rename script that had clobbered an anti-pattern example, an over-claim importing the *Unreleased* CHANGELOG's QUERY-into-3.2 behavior as if shipped, and the `remove` signature break.
- **0.1.3** (2026-06-10) — adversarial-review precision pass (cache key parts, limiter 413 misattribution dropped, SSE `TryStream` item type).
- **0.1.2** (2026-06-10) — audit fixes (Timeout 503, no `force-host`/`parse_msgpack`, ACME :80 join) + new coverage: `#[craft]`, OpenTelemetry (incl. the `opentelemetry = "0.31"` version-match gotcha), response cache, tus, Unix socket / multi-listener, `{**}`/`{*+}`/`{*?}` wildcard semantics, `StaticDir::exclude` / `compressed_variation`.
- **0.1.1** — Context7 + cargo-check verification pass; packaged as installable plugin.

## Companion skills

- `soybean-admin-salvo` — wiring a Salvo backend to a soybean-admin (Vue3 + Naive UI) frontend.
- `rust-idioms` — Rust language-layer idioms/design/compiler-error diagnosis (the non-framework half).

Zero hooks / zero MCP / zero extra permissions — pure reference text loaded on demand.

## Install

- **As a plugin (marketplace):** `/plugin marketplace add waydone/salvo-skill`, then `/plugin install salvo-skill@salvo-skill`.
- **Local (symlink):** from the repo root, `ln -sfn "$PWD/skills/salvo-skill" ~/.claude/skills/salvo-skill`.

## Updating after an edit

Bump `version` in `.claude-plugin/{plugin,marketplace}.json`, `git push`, then `claude plugin marketplace update salvo-skill && claude plugin update salvo-skill@salvo-skill`, and restart. Without a version bump, `plugin update` reports "already at latest" and won't pull the new content.
