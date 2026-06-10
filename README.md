# salvo-skill

A Claude Code / agent skill for the **[Salvo](https://salvo.rs) Rust web framework** — target **0.93.0**. Install as a plugin (see below) or by symlinking into `~/.claude/skills/`.

Encodes Salvo's 0.93.0 API surface (project skeleton, routing, `#[handler]`/`#[craft]`, `Writer`/`Scribe`, extractors, `JwtAuth`, middleware/`Depot`, OpenAPI, WebSocket/SSE, static files & tus resumable uploads, response cache, OpenTelemetry, multi-listener/Unix socket, error handling, testing, ops) as a lean **hub-and-spoke** skill: a compact `SKILL.md` + on-demand `references/*.md` (×13), plus `evals/`.

## Discipline baked in

- **Verify with Context7 before non-trivial code** — Salvo's API shifts across minor versions; the skill mandates a Context7 doc check for anything beyond a trivial `Router`.
- **edition 2024 / Rust 1.85+** skeleton; for existing projects, match what's there.
- `0.93.0` is non-breaking over `0.92.2` (4 internal fixes noted in `SKILL.md`).

## Verification

Every load-bearing API across `SKILL.md` + all `references/*.md` was checked two ways (2026-06): (1) Context7 doc cross-verification of all 14 files, and (2) a real `cargo check` against salvo 0.93.0 that caught and fixed 11 compile-level inaccuracies (extractor imports not in prelude, `parse_queries` is sync, `filters::scheme(Scheme::HTTPS)`, CORS `allow_headers(vec!…)`, `AllowOrigin::any()`, `SecureMaxSize::new`, `default_parameter_in`, …) — then re-compiled to 0 errors.

The v0.1.3 round (2026-06-10) added three more layers: a **content audit** against crate sources (fixed: `Timeout` default is **503** not 408; removed the nonexistent `force-host` feature and `parse_msgpack`; ACME HTTP-01 needs `.join(TcpListener :80)`), **`cargo run` verification** of every newly added snippet (craft / cache / otel / tus / wildcards / StaticDir builder), and an **adversarial review** — 25 claims attacked against local 0.93.0 crate sources, 0 refuted, 5 precision-fixed (e.g. SSE streams must be `TryStream<Ok = SseEvent>`; `RequestIssuer::default()` keys on all five URL parts). Bonus find: salvo-otel's own doc example for `Tracing::new()` is wrong upstream — this skill has the correct signature.

## Changelog

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
