# MutaMarket rewrite

Ground-up rewrite of MutaMarket (mutamarket.com — marketplace and toolbox
for abyssal modules in EVE Online) from Laravel 13 + Inertia/Vue to
**Rust: Axum + Postgres (sqlx) as a pure JSON API**, with a **SvelteKit
(SSR, adapter-node) frontend in `frontend/`**. A shared origin (Vite dev
proxy locally, Caddy in production) routes backend paths to Axum and
everything else to SvelteKit, so cookies always stay same-origin — never
point the browser at Axum's port directly.

The legacy Laravel project (private, not needed to build or run this
repository) is the **specification** wherever one is available locally; its
paths below are relative to that checkout. The rewrite's goal is feature
parity with it.

## The prime rule: legacy first, in fine detail

Before implementing anything, read how the legacy solved it — and mirror it:

- **Routes**: `routes/web.php` and `routes/api.php` define every endpoint,
  parameter pattern and middleware. `tests/integration/routes.rs` carries the inventory
  as status-contract tests; failing groups are the porting backlog.
- **JSON resources**: `app/Http/Resources/*` define the exact field names
  and nesting we must emit. `whenLoaded` relations plus the controller's
  relation loadout (`ModuleBuilder::withDefaultRelations` etc.) determine
  the key set per endpoint. Loaded-but-empty relations are
  present-and-null keys, unloaded ones are absent.
- **Controllers/Actions/Jobs**: `app/Http/Controllers`, `app/Actions`,
  `app/Jobs` hold the behavior, including quirks (PHP truthiness, null
  coalescing, exact error messages and status codes). Port quirks
  faithfully and document them where they live.
- **UI**: the SvelteKit frontend mirrors the legacy Vue components
  (`resources/js/Pages`, `resources/js/Components`) in layout structure and
  behavior — formatting (`resources/js/Helper/AttributeFormatter.ts`),
  display-setting cookies, bar modes, masonry grid row spans. Read the
  actual Vue component before building its Svelte counterpart
  (`frontend/src/routes`, `frontend/src/lib/components`).
  **Deliberate exception**: colors, radius and typography follow the
  shadcn-svelte theme in `frontend/src/routes/layout.css` (mira style,
  mist base, Outfit font) instead of the legacy palette — a redesign
  chosen at the SvelteKit pivot. Domain-specific styling (roll-bar
  colors, gold/diamond gradients, card layers) still ports from legacy.
- **Fixtures are the spec for math**: `tests/fixtures/module_parsing`
  (445 modules with exact expected outputs) and `tests/fixtures/reference`
  are legacy exports. The mutation math must match at 1e-9 relative
  tolerance; never regenerate fixtures to make a failing change pass.

## Testing rules

- Every increment lands **with its tests** in the same commit(s).
- Tests must be **precise**: assert exact JSON key sets (sorted-keys
  comparisons at every nesting level), exact legacy error messages and
  status codes, exact orderings — not just spot values.
- Characterization suites (`module_parsing`, `sde_pipeline`,
  `module_ingestion`) pin the math against legacy snapshots; keep them
  green at all times.
- Behavior tests drive the real router (axum `oneshot`), with external
  services (ESI, EVE SSO) replaced by local mock servers on ephemeral
  ports — never by stubbing our own code.
- Integration suites are modules of the single `tests/integration` binary
  and run single-threaded (`RUST_TEST_THREADS=1` in `.cargo/config.toml`):
  suites clean the shared tables they assert about and a few mutate
  process env vars (scoped with `common::EnvGuard`), so tests must never
  run concurrently. Run one suite with `cargo test <module>::`.
- DB tests run against the dedicated `mutamarket_test` database
  (`db::test_pool()`, created automatically; `TEST_DATABASE_URL` to
  override) so seeding never wipes development data. Test setups must be
  idempotent across runs and suites — clean the rows they assert about.
- Run `cargo test` and `cargo clippy --all-targets` (zero warnings) before
  committing.

## Workflow rules

- **Small, coherent commits** with conventional-commit prefixes; the
  history is the migration log.
- **No magic numbers**: every literal EVE id, threshold or tuning value
  becomes a named `const` with a doc comment saying what it is and why.
- **No legacy dependence in production paths**: legacy dumps are test
  fixtures only; production data comes from the native SDE import
  (`cargo run --bin sde_import`) and ESI. Deliberate exception: the
  one-time bootstrap `LEGACY_IMPORT_CONFIRM=1 cargo run --bin
  legacy_import` (src/legacy) wipes the domain tables and replays the
  legacy MySQL snapshot (`LEGACY_DATABASE_URL`); reference tables stay
  SDE-owned and the live market is rebuilt from ESI afterwards.
- Keep imports **minimal**: only the tables and columns a ported feature
  actually uses; extend by migration when the next feature needs more.
- Divergences from legacy must be deliberate and documented in place
  (e.g. axum redirect codes, retired ESI scopes, Postgres null ordering).

## Layout and commands

- `src/mutation/` — roll-quality math (characterization-tested, pure).
- `src/sde/` — native SDE import pipeline; `src/db/` — pool/migrations/seed.
- `src/modules/` — module domain: ingest, queries, search, view DTOs.
- `src/auth/` — EVE SSO (JWKS-verified JWTs, owner-hash identity), sessions.
- `src/server/` — axum router, JSON API, display settings; `src/view/` —
  serializable response DTOs shared with the frontend.
- `frontend/` — the SvelteKit app (SSR, adapter-node, Tailwind v4,
  shadcn-svelte); its Vite dev server proxies backend paths to Axum.
- `assets/` — everything the API ships beside the binary: `img/` (served
  at `/img`), `fonts/` (compiled into the OG renderer), `docs/` (the
  documentation pages' markdown). `deploy/` — the shared-origin
  Caddyfiles (compose dev origin and production).
- Dev runs via solo: `cargo run` (API on `BIND_ADDR`, default
  127.0.0.1:3000) plus `npm run dev` in `frontend/`, Postgres via docker
  compose. `.env` for config (see `.env.example`); EVE SSO login needs
  `EVE_CLIENT_ID`/`EVE_CLIENT_SECRET`.
- One-command stack: `docker compose up --build` (solo process `Stack`)
  serves the whole app on the fixed origin http://localhost:5100 and
  bootstraps the SDE automatically (downloads the latest build, seeds
  only when it changed; `SDE_FORCE=1` reseeds).
- Frontend tests are vitest suites landing with their code in the same
  commit; the Rust suite pins the JSON contracts, SvelteKit owns page
  rendering. All mutations call Axum via fetch — no SvelteKit form
  actions (the proxy sends every non-GET to Axum).
