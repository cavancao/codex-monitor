# Cross-Machine Data Adapter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the machine-specific Codex data reader with a read-only, schema-aware adapter that discovers sources dynamically and degrades per field.

**Architecture:** Small Rust adapters produce typed field candidates from discovered roots, SQLite databases, rollout logs, and auth claims. A snapshot aggregator selects candidates per field by freshness and confidence, while a diagnostics writer persists only redacted structure metadata in the Tauri application data directory.

**Tech Stack:** Rust 2021, Tauri 2, rusqlite, serde/serde_json, chrono, walkdir, tempfile, Vue 3, TypeScript, Vitest.

## Global Constraints

- Keep the existing Vue HUD, tray, window lifecycle, `CodexStatus`, and Tauri command/event contract.
- Read only the current user's local files; never modify Codex data.
- Prefer `CODEX_HOME`, then score current-user candidates by structure; never hard-code usernames, drive letters, or installation paths.
- Open SQLite with `SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_NO_MUTEX`; execute only built-in read queries.
- Do not persist tokens, complete email addresses, conversation bodies, or credential values.
- Scan at most 240 candidate files, depth at most 6, without following directory links.
- Read at most the final 256 KiB of an individual rollout log.
- Production `auto` mode must show `--` when no trustworthy source exists and must never fall back to Mock.

---

### Task 1: Dynamic source discovery

**Files:**
- Create: `src-tauri/src/data_sources/mod.rs`
- Create: `src-tauri/src/data_sources/discovery.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/data_sources/discovery.rs`

**Interfaces:**
- Produces: `DiscoveryInputs { home: PathBuf, codex_home: Option<PathBuf>, system_roots: Vec<PathBuf> }`
- Produces: `DataRootCandidate { path: PathBuf, score: u16, evidence: Vec<RootEvidence> }`
- Produces: `discover_roots(inputs: &DiscoveryInputs) -> Vec<DataRootCandidate>`
- Consumes later: SQLite, rollout, auth, recon, and snapshot adapters use the ordered candidates.

- [ ] **Step 1: Write failing discovery tests**

```rust
fn fixture_root(path: &Path) {
    std::fs::create_dir_all(path.join("sessions")).unwrap();
    std::fs::write(path.join("models_cache.json"), "{}").unwrap();
    rusqlite::Connection::open(path.join("state_1.sqlite")).unwrap();
}

#[test]
fn codex_home_has_priority_without_a_fixed_directory_name() {
    let temp = tempfile::tempdir().unwrap();
    let custom = temp.path().join("portable-data");
    fixture_root(&custom);
    let inputs = DiscoveryInputs {
        home: temp.path().join("home"),
        codex_home: Some(custom.clone()),
        system_roots: vec![],
    };
    assert_eq!(discover_roots(&inputs)[0].path, custom);
}

#[test]
fn discovers_nested_structural_candidate_and_deduplicates_it() {
    let temp = tempfile::tempdir().unwrap();
    let candidate = temp.path().join("Local").join("client-state");
    fixture_root(&candidate);
    let inputs = DiscoveryInputs {
        home: temp.path().to_path_buf(),
        codex_home: Some(candidate.clone()),
        system_roots: vec![temp.path().join("Local")],
    };
    assert_eq!(discover_roots(&inputs).iter().filter(|v| v.path == candidate).count(), 1);
}
```

- [ ] **Step 2: Verify RED**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml data_sources::discovery
```

Expected: compilation fails because `DiscoveryInputs` and `discover_roots` do not exist.

- [ ] **Step 3: Implement bounded structural discovery**

Implement evidence for `sessions`, `models_cache.json`, `auth.json`, `state_*.sqlite`, and recent `.jsonl` files. Canonicalize only existing directories, do not follow links, search `home` only to depth 2 and system roots only to depth 4, then sort by explicit `CODEX_HOME`, score, and newest evidence.

- [ ] **Step 4: Verify GREEN**

Run the Task 1 command and expect all discovery tests to pass.

- [ ] **Step 5: Commit**

```powershell
git add src-tauri/src/data_sources src-tauri/src/lib.rs
git commit -m "Add dynamic Codex data discovery"
```

### Task 2: Schema-aware SQLite adapter

**Files:**
- Create: `src-tauri/src/data_sources/candidate.rs`
- Create: `src-tauri/src/data_sources/sqlite.rs`
- Modify: `src-tauri/src/data_sources/mod.rs`
- Test: `src-tauri/src/data_sources/sqlite.rs`

**Interfaces:**
- Produces: `FieldCandidate<T> { value: T, source: &'static str, observed_at: DateTime<Utc>, confidence: f32 }`
- Produces: `ThreadCandidates { model, reasoning_effort, client_version, rollout_paths }`
- Produces: `read_thread_candidates(root: &Path) -> AdapterResult<ThreadCandidates>`
- `AdapterResult<T>` contains `value: T` and redacted `issues: Vec<DiagnosticIssue>`.

- [ ] **Step 1: Write failing SQLite fixture tests**

```rust
struct SqliteFixture(tempfile::TempDir);

impl SqliteFixture {
    fn path(&self) -> &Path { self.0.path() }
}

fn sqlite_fixture(schema_and_rows: &str) -> SqliteFixture {
    let temp = tempfile::tempdir().unwrap();
    let connection = rusqlite::Connection::open(temp.path().join("state_1.sqlite")).unwrap();
    connection.execute_batch(schema_and_rows).unwrap();
    drop(connection);
    SqliteFixture(temp)
}

#[test]
fn missing_tokens_column_does_not_hide_model_and_effort() {
    let fixture = sqlite_fixture(
        "CREATE TABLE threads(model TEXT, reasoning_effort TEXT, cli_version TEXT, updated_at_ms INTEGER);
         INSERT INTO threads VALUES('gpt-5.6-sol', 'high', '1.2.3', 1000);"
    );
    let result = read_thread_candidates(fixture.path()).unwrap();
    assert_eq!(result.value.model.unwrap().value, "gpt-5.6-sol");
    assert_eq!(result.value.reasoning_effort.unwrap().value, "high");
}

#[test]
fn supports_version_alias_without_requiring_all_known_columns() {
    let fixture = sqlite_fixture(
        "CREATE TABLE threads(model_slug TEXT, effort TEXT, app_version TEXT, updated_at INTEGER);
         INSERT INTO threads VALUES('gpt-5.6-sol', 'medium', '2.0.0', 2000);"
    );
    let result = read_thread_candidates(fixture.path()).unwrap();
    assert_eq!(result.value.client_version.unwrap().value, "2.0.0");
}
```

- [ ] **Step 2: Verify RED**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml data_sources::sqlite
```

Expected: compilation fails because the adapter types and function are missing.

- [ ] **Step 3: Implement schema inspection and per-column queries**

Read table names through `sqlite_master`, inspect columns with `PRAGMA table_info`, map exact supported aliases, quote identifiers only after allow-list selection, and build a `SELECT` containing only existing columns. Exclude `codex-auto-review` when a model column exists. Open every database read-only and continue to the next candidate after a schema mismatch.

- [ ] **Step 4: Verify GREEN**

Run the Task 2 command and expect all SQLite adapter tests to pass.

- [ ] **Step 5: Commit**

```powershell
git add src-tauri/src/data_sources
git commit -m "Read Codex SQLite schemas defensively"
```

### Task 3: Bounded rollout and auth adapters

**Files:**
- Create: `src-tauri/src/data_sources/rollout.rs`
- Create: `src-tauri/src/data_sources/auth.rs`
- Modify: `src-tauri/src/data_sources/mod.rs`
- Test: `src-tauri/src/data_sources/rollout.rs`
- Test: `src-tauri/src/data_sources/auth.rs`

**Interfaces:**
- Produces: `RolloutCandidates { model, reasoning_effort, speed_mode, subscription, remaining_percent, reset_at, client_version }`
- Produces: `discover_rollouts(root: &Path, hinted_paths: &[PathBuf]) -> Vec<PathBuf>`
- Produces: `read_rollout_candidates(root: &Path, paths: &[PathBuf]) -> AdapterResult<RolloutCandidates>`
- Produces: `AuthCandidates { username, subscription }`
- Produces: `read_auth_candidates(root: &Path) -> AdapterResult<AuthCandidates>`

- [ ] **Step 1: Write failing rollout tests**

```rust
#[test]
fn newer_rate_event_wins_across_multiple_rollouts() {
    let fixture = rollout_fixture(&[
        ("old.jsonl", token_count_event("free", 40.0, 1000), 1000),
        ("new.jsonl", token_count_event("plus", 12.0, 2000), 2000),
    ]);
    let paths = discover_rollouts(fixture.path(), &[]);
    let result = read_rollout_candidates(fixture.path(), &paths);
    assert_eq!(result.value.subscription.unwrap().value, "plus");
    assert_eq!(result.value.remaining_percent.unwrap().value, 88.0);
}

#[test]
fn invalid_lines_do_not_discard_valid_tail_events() {
    let fixture = rollout_text_fixture("bad json\n{\"payload\":{\"type\":\"token_count\",\"rate_limits\":{\"primary\":{\"used_percent\":5,\"resets_at\":2000}}}}\n");
    let result = read_rollout_candidates(fixture.path(), &discover_rollouts(fixture.path(), &[]));
    assert_eq!(result.value.remaining_percent.unwrap().value, 95.0);
}
```

- [ ] **Step 2: Write failing auth redaction tests**

```rust
#[test]
fn invalid_or_valid_tokens_never_enter_diagnostics() {
    let fixture = auth_fixture("header.secret_payload.signature");
    let result = read_auth_candidates(fixture.path());
    let serialized = serde_json::to_string(&result.issues).unwrap();
    assert!(!serialized.contains("secret_payload"));
}
```

- [ ] **Step 3: Verify RED**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml data_sources::rollout
cargo test --manifest-path src-tauri/Cargo.toml data_sources::auth
```

Expected: compilation fails because rollout/auth functions are missing.

- [ ] **Step 4: Implement bounded tail parsing and in-memory JWT claims**

Discover `.jsonl` files to depth 6 without following links, cap results at 240 by modification time, read only the final 256 KiB, and parse each line independently. Accept known event shapes only. Decode JWT payload in memory, return only name/email display and plan claim candidates, and emit issue codes without credential text.

- [ ] **Step 5: Verify GREEN**

Run both module test filters separately and expect all tests to pass.

- [ ] **Step 6: Commit**

```powershell
git add src-tauri/src/data_sources
git commit -m "Add bounded Codex log and auth adapters"
```

### Task 4: Per-field aggregation and redacted diagnostics

**Files:**
- Create: `src-tauri/src/data_sources/diagnostics.rs`
- Create: `src-tauri/src/data_sources/snapshot.rs`
- Modify: `src-tauri/src/status.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/data_sources/snapshot.rs`
- Test: `src-tauri/src/data_sources/diagnostics.rs`

**Interfaces:**
- Produces: `SnapshotOptions { force_discovery: bool, diagnostics_path: Option<PathBuf> }`
- Produces: `collect_snapshot(inputs: &DiscoveryInputs, options: &SnapshotOptions) -> Result<CodexStatus, String>`
- Produces: `write_diagnostics(path: &Path, report: &DiagnosticReport) -> Result<(), String>`
- Produces: `SnapshotCandidates` with one `Vec<FieldCandidate<T>>` per `CodexStatus` field.
- Produces: `build_status(candidates: SnapshotCandidates) -> CodexStatus`.
- Consumes all adapters from Tasks 1–3.

- [ ] **Step 1: Write failing aggregation tests**

```rust
#[test]
fn recent_rate_plan_overrides_older_free_auth_claim() {
    let mut candidates = SnapshotCandidates::default();
    candidates.subscription.push(FieldCandidate::new("free".into(), "auth", utc(1000), 0.80));
    candidates.subscription.push(FieldCandidate::new("plus".into(), "rollout", utc(2000), 0.98));
    let status = build_status(candidates);
    assert_eq!(status.subscription.value.as_deref(), Some("plus"));
}

#[test]
fn missing_sqlite_effort_does_not_hide_rollout_quota() {
    let mut candidates = SnapshotCandidates::default();
    candidates.remaining_percent.push(FieldCandidate::new(87.0, "rollout", utc(2000), 0.98));
    let status = build_status(candidates);
    assert_eq!(status.reasoning_effort.value, None);
    assert_eq!(status.remaining_percent.value, Some(87.0));
    assert_eq!(status.sync_state, "connected");
}
```

- [ ] **Step 2: Write failing diagnostics safety test**

```rust
#[test]
fn report_contains_structure_but_not_values_or_home_prefix() {
    let report = diagnostic_fixture();
    let json = serde_json::to_string(&report.redacted_for(Path::new("C:/Users/Alice"))).unwrap();
    assert!(json.contains("threads"));
    assert!(!json.contains("Alice"));
    assert!(!json.contains("alice@example.com"));
    assert!(!json.contains("eyJ"));
}
```

- [ ] **Step 3: Verify RED**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml data_sources::snapshot
cargo test --manifest-path src-tauri/Cargo.toml data_sources::diagnostics
```

Expected: compilation fails because the aggregator and diagnostics writer are missing.

- [ ] **Step 4: Implement candidate selection and diagnostic serialization**

Select candidates per field by field-specific source priority, then observation time, then confidence. Return `connected` when any core field is available and `recon-required` only when all adapters yield no fields. Serialize root evidence, table/column names, event type names, issue codes, and masked relative paths only.

- [ ] **Step 5: Verify GREEN**

Run both Task 4 module filters separately and expect all tests to pass.

- [ ] **Step 6: Commit**

```powershell
git add src-tauri/src/data_sources src-tauri/src/status.rs src-tauri/src/lib.rs
git commit -m "Aggregate Codex fields with redacted diagnostics"
```

### Task 5: Tauri refresh and recon integration

**Files:**
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/src/discovery.rs`
- Modify: `src-tauri/src/real.rs`
- Modify: `README.md`
- Test: `src-tauri/src/main.rs`
- Test: `src-tauri/src/real.rs`

**Interfaces:**
- `get_status(app, "auto")` calls the new collector with cached discovery.
- `run_recon(app)` forces discovery, writes redacted diagnostics, emits `recon-finished`, then emits the fresh `status-changed`.
- Existing explicit `field-mapping.json` remains a lower-priority optional source.

- [ ] **Step 1: Write failing integration tests**

```rust
#[test]
fn force_refresh_collects_from_newly_available_root() {
    let fixture = RefreshFixture::new();
    assert_eq!(fixture.collect(false).sync_state, "recon-required");
    fixture.add_valid_root();
    assert_eq!(fixture.collect(true).model.value.as_deref(), Some("gpt-5.6-sol"));
}

#[test]
fn integration_fixture_does_not_depend_on_real_user_home() {
    let fixture = CompleteFixture::new();
    let status = fixture.snapshot();
    assert_eq!(status.subscription.value.as_deref(), Some("plus"));
    assert_eq!(status.remaining_percent.value, Some(87.0));
}
```

- [ ] **Step 2: Verify RED**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml force_refresh_collects
cargo test --manifest-path src-tauri/Cargo.toml integration_fixture
```

Expected: tests fail because the commands still call the old single-root `real::snapshot`.

- [ ] **Step 3: Integrate the collector**

Replace `real::snapshot()` internals with the new collector, route recon roots through shared discovery, pass the Tauri app data diagnostics path, and keep explicit mapping fallback for fields still null. Update README to describe automatic refresh, `CODEX_HOME`, diagnostic location, and per-field degradation.

- [ ] **Step 4: Verify GREEN**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
npm test
```

Expected: all Rust and frontend tests pass.

- [ ] **Step 5: Commit**

```powershell
git add src-tauri/src README.md
git commit -m "Integrate adaptive Codex data collection"
```

### Task 6: Production verification and installer

**Files:**
- Modify only if verification exposes a regression.
- Build: `Codex运行情况_0.1.0_x64-setup.exe`

**Interfaces:**
- Consumes the completed application.
- Produces a verified Windows NSIS installer in the project root.

- [ ] **Step 1: Run complete checks**

```powershell
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: zero failed tests and all commands exit 0.

- [ ] **Step 2: Build the NSIS installer**

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
npm run tauri -- build --bundles nsis
```

Expected: Tauri reports one NSIS bundle under `src-tauri/target/release/bundle/nsis`.

- [ ] **Step 3: Replace and verify the root installer**

Copy the generated NSIS file to `Codex运行情况_0.1.0_x64-setup.exe`, calculate SHA-256 with `Get-FileHash`, and verify `src-tauri/target/release/codex-monitor.exe` has PE subsystem value `2`.

- [ ] **Step 4: Commit any documentation-only verification updates**

```powershell
git status --short
```

Do not add the untracked installer binary to Git. Commit only source or documentation changes required by a verified regression fix.
