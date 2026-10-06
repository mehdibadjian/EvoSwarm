# Gateway

`evoswarm-gateway` is the **optional** transparent reverse proxy (Axum/Tokio) that sits between Claude Code and the upstream model API. Point `ANTHROPIC_BASE_URL` at it and it streams SSE through, audits spend, and (when wired) injects retrieved exemplars — all while staying under a tight time-to-first-token budget.

## Modules

| Module | Role |
|---|---|
| `proxy.rs` (~352 lines) | Byte-for-byte SSE pass-through to the upstream; forwards the stream and propagates upstream errors. `e4-1` review (SEAM): verified against a **stub upstream** for byte fidelity, error propagation, and structural abort; live-provider pass-through needs keys. |
| `usage.rs` (~327 lines) | A "tee" that observes the SSE stream, extracts token/cost accounting, and writes it to a local SQLite DB (`usage.db`). `e4-2` review (SEAM): verified against stub SSE frames + local sqlite; live accounting needs keys. Surfaced by `evoswarm usage` — see [CLI](CLI.md). |
| `injection.rs` | Enriches outgoing prompts with retrieved past exemplars, with a header/config **opt-out** bypass and a zero-extra-call fast path. `e4-4` review (SEAM): the opt-out and fast path are verified; the injector itself is a test stub because live exemplar retrieval (`e4-3`, FalkorDB) is **blocked** on `e2-5`. |
| `latency.rs` | Measures TTFT overhead. `e4-5` review (SEAM): the 20-stream budget is measured loopback-vs-loopback (added p95 ~2 ms ≤ 50 ms target); the IDE-vs-provider budget needs a live provider. |
| `config.rs` | Gateway configuration. |

## Operating contract

- **Adds < 50 ms to TTFT at p95** under 20 concurrent streams, with byte-for-byte stream fidelity.
- **Exemplar injection is best-effort and bounded** — injected only if the memory lookup returns within ~20 ms, and always opt-out-able, so a slow/absent memory store can't stall the stream.

## Status summary

The gateway's *proxying, accounting, injection-bypass, and latency* logic all exist and are tested against stubs (four stories at `review`/SEAM). What's genuinely deferred is anything that requires a **live upstream model** (real accounting, real pass-through under load) and **live FalkorDB exemplars** (`e4-3`, backlog — blocked on the memory epic). See [Roadmap & Status](Roadmap-and-Status.md).
