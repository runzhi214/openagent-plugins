# BUGS.md — Known Issues & Technical Debt

> Last updated 2026-07-31 (rev 1).
> Format: `[P0]` = critical, `[P1]` = high, `[P2]` = medium, `[P3]` = low.
> `[DEBT]` = technical debt (no immediate breakage, will compound).

---

## 🐛 Bugs

### [P2] OpenViking Search API hangs for certain newly created sessions

[openviking-memory/src/lib.rs](openviking-memory/src/lib.rs): The `memory_search` function calls `openviking_common::search`, which sends `POST /api/v1/search/search` to the OpenViking server. For certain newly created sessions (where messages were just mirrored via `AddMessage`), the search API accepts the connection but never responds — the HTTP request hangs indefinitely.

**Root cause**: OpenViking server-side issue. The search endpoint appears to deadlock when querying a session whose embedding index is still being built or in an inconsistent state. Other sessions (older, already indexed) respond normally within milliseconds. The issue is intermittent and not reproducible on every new session.

**Impact**: The built-in `recall` tool calls `Memory.Search` → `layeredMemory.Search` → `plugin.Search` → `memory_search` → `http_request`. A hanging search holds the WASM mutex (`wasmMemory.mu`), blocking all subsequent WASM calls (including `memory_append` mirrors). The openagent server remains responsive for non-WASM endpoints (e.g. `GET /models`), but any agent turn involving memory operations stalls.

**Mitigation (already applied)**: A 30s HTTP client timeout was added to `plugin/wasmhost/hostapi_std.go` (`NewHTTPClient` now uses `&http.Client{Timeout: 30 * time.Second}` instead of `http.DefaultClient`). This prevents indefinite hangs — the search now times out after 30s, releases the WASM mutex, and `layeredMemory.Search` falls back to SQLite FTS5. The user sees a delayed response instead of a permanent hang.

**Remaining follow-up**:
1. File upstream bug report against OpenViking server with repro steps (create session → AddMessage → immediately Search → hang).
2. Consider adding a session-level "warming up" flag in the plugin: skip search for sessions created < N seconds ago, return empty results immediately instead of risking a hang.
3. Consider reducing the HTTP timeout for search specifically (e.g. 10s) vs. append (30s), since search has a SQLite fallback but append does not.

Repro:
1. Start openagent with `openviking-memory` plugin loaded
2. Send a message (triggers `memory_append` → `AddMessage` → OV session auto-created)
3. Immediately ask the agent to use `recall` tool (triggers `memory_search` → `Search`)
4. Search hangs — `tool.execute` stage enters but never leaves
5. Direct `curl` to `POST /api/v1/search/search` with the same `session_id` also hangs
6. `curl` with a different (older) `session_id` responds normally

Evidence:
- Session `659055059f201d4650d010cc2c38a547`: search hangs (>60s observed)
- Session `20260730-124324-bfe90c26b797496e`: search returns in <1s
- OpenViking `CreateSession` and `AddMessage` APIs remain responsive throughout

---

## Legend

| Tag | Meaning |
|-----|---------|
| `P0` | Critical — data loss, API contract violation, resource leak |
| `P1` | High — incorrect behavior in common scenarios |
| `P2` | Medium — incorrect behavior in edge cases |
| `P3` | Low — cosmetic or harmless |
| `DEBT` | Technical debt — will compound as codebase grows |
