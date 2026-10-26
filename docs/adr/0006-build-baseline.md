# ADR-0006: Build baseline — edition 2024, rustls, stderr-only logging

Status: accepted (2026-10-27)

## Context

stdio MCP servers share a process channel with JSON-RPC on stdout: any stray write corrupts
the protocol. Upstream used urllib with `ssl.create_default_context()` (OS trust store).
Toolchain here is rustc 1.99.

## Decision

- `edition = "2024"`, `rust-version` pinned to the MSRV of the locked `rmcp` version (and
  raised only when rmcp raises it).
- HTTP client: `reqwest` (current 0.13.x) with the `rustls` feature (`rustls-platform-verifier`, i.e. the OS trust store); no
  OpenSSL, no `native-tls`.
- Logging: `tracing` + `tracing-subscriber`, writer pinned to **stderr**, `EnvFilter` from
  `RUST_LOG`, default `info`. Request logs carry method + path + status; `Authorization` and
  token values are never logged (redaction is a test, not a promise).
- Async runtime: `tokio` (rmcp requirement), `#[tokio::main]` in `main` only.
- CI: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test
  --workspace` on ubuntu + macos; release binaries built on tag (linux-gnu, linux-musl,
  macos-arm64).

## Consequences

- musl static builds work without an OpenSSL toolchain.
- Breaking rustc 1.99 assumptions requires bumping both MSRV and CI images.
- A test asserts stdout stays empty in stdio mode (guards against accidental `println!`).
