# salvo-skill

A Claude Code / agent skill for the **[Salvo](https://salvo.rs) Rust web framework** — target **0.93.0**. Install by symlinking into `~/.claude/skills/`.

Encodes Salvo's 0.93.0 API surface (project skeleton, routing, `#[handler]`, `Writer`/`Scribe`, extractors, `JwtAuth`, middleware/`Depot`, OpenAPI, WebSocket/SSE, static files, error handling, testing, ops) as a lean **hub-and-spoke** skill: a compact `SKILL.md` + on-demand `references/*.md`, plus `evals/`.

## Discipline baked in

- **Verify with Context7 before non-trivial code** — Salvo's API shifts across minor versions; the skill mandates a Context7 doc check for anything beyond a trivial `Router`.
- **edition 2024 / Rust 1.85+** skeleton; for existing projects, match what's there.
- `0.93.0` is non-breaking over `0.92.2` (4 internal fixes noted in `SKILL.md`).

## Verification

Every load-bearing API across `SKILL.md` + all `references/*.md` was checked two ways (2026-06): (1) Context7 doc cross-verification of all 14 files, and (2) a real `cargo check` against salvo 0.93.0 that caught and fixed 11 compile-level inaccuracies (extractor imports not in prelude, `parse_queries` is sync, `filters::scheme(Scheme::HTTPS)`, CORS `allow_headers(vec!…)`, `AllowOrigin::any()`, `SecureMaxSize::new`, `default_parameter_in`, …) — then re-compiled to 0 errors.

## Companion skills

- `soybean-admin-salvo` — wiring a Salvo backend to a soybean-admin (Vue3 + Naive UI) frontend.
- `rust-idioms` — Rust language-layer idioms/design/compiler-error diagnosis (the non-framework half).

Zero hooks / zero MCP / zero extra permissions — pure reference text loaded on demand.

## Install

- **As a plugin (marketplace):** `/plugin marketplace add waydone/salvo-skill`, then `/plugin install salvo-skill@salvo-skill`.
- **Local (symlink):** `ln -sfn /Users/coco/Documents/code/salvo/salvo-skill/skills/salvo-skill ~/.claude/skills/salvo-skill`.
