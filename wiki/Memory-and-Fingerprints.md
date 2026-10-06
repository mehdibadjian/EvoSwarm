# Memory & Fingerprints

`evoswarm-memory` holds the primitives behind EvoSwarm's "remember what worked" promise: a content-addressed blob store for large artifacts, and deterministic fingerprints that decide when a past winner can be replayed with zero model spend.

## Blob store (`blob_store.rs`)

An immutable, content-addressed disk store rooted at `.evoswarm/blobs/`.

- **`write(data)`** — SHA-256 the bytes, store at `<root>/<hash>` if not already present (dedup is automatic — identical content shares one blob), update its last-referenced timestamp, and return the hash.
- **`read(hash)`** — returns the bytes or `BlobError::NotFound`.
- **`set_last_referenced` / `get_last_referenced`** — a sidecar `<hash>.lastref` file records a Unix timestamp per blob.
- **`gc(now, referenced)`** — garbage collection: deletes blobs that are (a) not in the `referenced` set **and** (b) last referenced more than the retention window ago (**7 days**). Returns a `GcReport { deleted_count, freed_bytes }`. `.lastref` sidecars and still-referenced or in-window blobs are skipped.

This gives the search loop a cheap place to stash diffs, execution logs, and build outputs keyed by content, with age-based eviction that never touches data the graph still points at.

Status: `e2-1` is **done** and fully deterministic (pure file + hash + time logic, tested with temp dirs).

## Fingerprints (`fingerprints.rs`)

The triple fingerprint that gates replay:

- **`repo_fingerprint(touched_paths, lockfile)`** — SHA-256 over the *contents of the touched files in sorted order* (for determinism), then the lockfile if it exists. Result is a hex digest.
- **`toolchain_fingerprint(versions)`** — SHA-256 over `compiler|runtime|harness` joined with a deterministic separator.

Together with a spec hash (the task definition), a replay only fires when all three match — so a stored winner is never served against a materially different repo or toolchain.

Status: `e2-3` is **done** — the hashing is pure and tested. **Graph-backed replay itself is not yet implemented:** writing/reading lineage to FalkorDB (`e2-2`) and verified replay (`e2-4`) are roadmap-declared **blocked** stories (currently plain `backlog` in the ledger) because they need a running FalkorDB. The primitives are here; the graph that uses them isn't wired yet.

## The dual-store design

The README describes a dual store: **FalkorDB** holds the graph (lineage edges, scores, a vector index for similarity) while this disk blob store holds the *bytes*, referenced by hash. The split keeps the graph small (pointers + scores) and the blob store immutable and deduplicated.

```
FalkorDB graph node ──(hash pointer)──▶ BlobStore blob  (patch / logs / build output)
```

## What's proven vs. deferred

| Capability | Story | State |
|---|---|---|
| Content-addressed blobs + GC | `e2-1` | done (deterministic) |
| Repo + toolchain fingerprints | `e2-3` | done (pure hashing) |
| seccomp as an isolation layer (sibling primitive) | `e2-7` | done |
| Write lineage to FalkorDB | `e2-2` | backlog — needs FalkorDB |
| Verified replay (serve stored winner) | `e2-4` | backlog — needs FalkorDB |
| Seeding from similar winners (vector) | `e2-5` | backlog — needs FalkorDB |
| Approve candidate/adversary tests | `e2-6` | backlog |
| Lineage CLI surface | `e2-8` | backlog |

See [The Crucible](The-Crucible.md) for the sandbox that re-verifies a replayed winner, and [Roadmap & Status](Roadmap-and-Status.md) for the full Epic 2 picture.
