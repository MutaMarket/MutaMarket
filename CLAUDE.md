# MutaMarket

The codebase behind mutamarket.com, the marketplace and toolbox for
abyssal modules in EVE Online: a **Rust API (Axum + Postgres via sqlx)**
serving pure JSON, and a **SvelteKit frontend (SSR, adapter-node) in
`frontend/`**. One shared origin (the Vite dev proxy locally, Caddy in
production) routes backend paths to Axum and everything else to
SvelteKit, so cookies stay same-origin. Never point the browser at Axum's
port directly.

## The legacy reference

MutaMarket ran for years as a Laravel + Inertia/Vue application. That
codebase is private and not needed to build or run this repository, but
it remains the reference for behavior: the site must keep doing what
users know. Where a checkout is available, read how it solved something
before changing or completing a feature, and mirror it in fine detail
(paths below are relative to that checkout):

- **Routes**: `routes/web.php` and `routes/api.php` define every endpoint,
  parameter pattern and middleware. `tests/integration/routes.rs` carries
  the inventory as status-contract tests.
- **JSON resources**: `app/Http/Resources/*` define the exact field names
  and nesting. `whenLoaded` relations plus the controller's relation
  loadout (`ModuleBuilder::withDefaultRelations` and friends) determine
  the key set per endpoint. Loaded-but-empty relations are
  present-and-null keys, unloaded ones are absent.
- **Controllers/Actions/Jobs**: `app/Http/Controllers`, `app/Actions`,
  `app/Jobs` hold the behavior, including quirks (PHP truthiness, null
  coalescing, exact error messages and status codes). Keep quirks
  faithfully and document them where they live.
- **UI**: the SvelteKit pages mirror the Vue components
  (`resources/js/Pages`, `resources/js/Components`) in layout structure
  and behavior: formatting (`resources/js/Helper/AttributeFormatter.ts`),
  display-setting cookies, bar modes, masonry grid row spans. Read the
  Vue component before touching its Svelte counterpart
  (`frontend/src/routes`, `frontend/src/lib/components`).
  **Deliberate exception**: colors, radius and typography follow the
  shadcn-svelte theme in `frontend/src/routes/layout.css` (mira style,
  mist base, Outfit font), a redesign chosen with the SvelteKit
  frontend. Domain-specific styling (roll-bar colors, gold/diamond
  gradients, card layers) still follows the original.
- **Fixtures are the spec for math**: `tests/fixtures/module_parsing`
  (445 modules with exact expected outputs) and `tests/fixtures/reference`
  are exports from the original application. The mutation math must
  match at 1e-9 relative tolerance; never regenerate fixtures to make a
  failing change pass.

Divergences from the original must be deliberate and documented in place
(for example axum redirect codes, retired ESI scopes, Postgres null
ordering).

## Testing rules

- Every increment lands **with its tests** in the same commit(s).
- Tests must be **precise**: assert exact JSON key sets (sorted-keys
  comparisons at every nesting level), exact error messages and status
  codes, exact orderings, not just spot values.
- Characterization suites (`module_parsing`, `sde_pipeline`,
  `module_ingestion`) pin the math against the reference snapshots; keep
  them green at all times.
- Behavior tests drive the real router (axum `oneshot`), with external
  services (ESI, EVE SSO) replaced by local mock servers on ephemeral
  ports, never by stubbing our own code.
- Integration suites are modules of the single `tests/integration` binary
  and run single-threaded (`RUST_TEST_THREADS=1` in `.cargo/config.toml`):
  suites clean the shared tables they assert about and a few mutate
  process env vars (scoped with `common::EnvGuard`), so tests must never
  run concurrently. Run one suite with `cargo test <module>::`.
- DB tests run against the dedicated `mutamarket_test` database
  (`db::test_pool()`, created automatically; `TEST_DATABASE_URL` to
  override) so seeding never wipes development data. Test setups must be
  idempotent across runs and suites: clean the rows they assert about.
- Frontend tests are vitest suites landing with their code in the same
  commit. The Rust suite pins the JSON contracts, SvelteKit owns page
  rendering.
- Run `cargo test` and `cargo clippy --all-targets` (zero warnings), and
  `npm test` plus `npm run check` in `frontend/`, before committing.

## Workflow rules

- **Small, coherent commits** with conventional-commit prefixes.
- **No magic numbers**: every literal EVE id, threshold or tuning value
  becomes a named `const` with a doc comment saying what it is and why.
- **Production data comes from the native SDE import**
  (`cargo run --bin sde_import`) and ESI. Reference dumps are test
  fixtures only. The one exception is the one-time bootstrap
  `LEGACY_IMPORT_CONFIRM=1 cargo run --bin legacy_import` (`src/legacy`),
  which wipes the domain tables and replays a MySQL snapshot of the
  original application (`LEGACY_DATABASE_URL`); reference tables stay
  SDE-owned and the live market is rebuilt from ESI afterwards.
- Keep imports **minimal**: only the tables and columns a feature
  actually uses; extend by migration when the next feature needs more.
- All frontend mutations call Axum via fetch, no SvelteKit form actions
  (the proxy sends every non-GET to Axum).

## Layout and commands

- `src/mutation/` roll-quality math (characterization-tested, pure).
- `src/sde/` native SDE import pipeline; `src/db/` pool, migrations, seed.
- `src/modules/` module domain: ingest, queries, search, view DTOs.
- `src/estimator/` the value estimator: random forests trained in-process,
  every model resident in memory.
- `src/auth/` EVE SSO (JWKS-verified JWTs, owner-hash identity), sessions.
- `src/server/` axum router, JSON API, display settings; `src/scheduler.rs`
  the background jobs; `src/view/` serializable response DTOs shared with
  the frontend.
- `frontend/` the SvelteKit app (SSR, adapter-node, Tailwind v4,
  shadcn-svelte); its Vite dev server proxies backend paths to Axum.
- `assets/` everything the API ships beside the binary: `img/` (served at
  `/img`), `fonts/` (compiled into the OG renderer), `docs/` (the
  documentation pages' markdown). `deploy/` the shared-origin Caddyfiles.
- Development: `cargo run` (API on `BIND_ADDR`, default 127.0.0.1:3000)
  plus `npm run dev` in `frontend/`, Postgres via docker compose. `.env`
  for config (see `.env.example`); EVE SSO login needs
  `EVE_CLIENT_ID`/`EVE_CLIENT_SECRET`.
- One-command stack: `docker compose up --build` serves the whole app on
  http://localhost:5100 and bootstraps the SDE automatically (downloads
  the latest build, seeds only when it changed; `SDE_FORCE=1` reseeds).
  `docker-compose.prod.yml` is the production override with Caddy.
