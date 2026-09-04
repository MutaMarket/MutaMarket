<p align="center">
  <img src="assets/img/MutaMarket.png" alt="MutaMarket" width="120">
</p>

<h1 align="center">MutaMarket</h1>

<p align="center">
  The marketplace and toolbox for abyssal modules in EVE Online.<br>
  <a href="https://mutamarket.com">mutamarket.com</a>
</p>

<p align="center">
  <a href="https://github.com/MutaMarket/MutaMarket/actions/workflows/ci.yml"><img src="https://github.com/MutaMarket/MutaMarket/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-a3e635" alt="MIT license"></a>
  <img src="https://img.shields.io/badge/Rust-2024-000000?logo=rust" alt="Rust 2024">
  <img src="https://img.shields.io/badge/Axum-0.8-000000" alt="Axum">
  <img src="https://img.shields.io/badge/SvelteKit-SSR-ff3e00?logo=svelte&logoColor=white" alt="SvelteKit">
  <img src="https://img.shields.io/badge/Postgres-17-4169e1?logo=postgresql&logoColor=white" alt="Postgres 17">
  <a href="https://discord.gg/FuwdBZ5cXq"><img src="https://img.shields.io/badge/Discord-join-5865f2?logo=discord&logoColor=white" alt="Discord"></a>
</p>

A Rust JSON API (Axum, Postgres via sqlx) and a SvelteKit frontend
(SSR, adapter-node). Everything runs on one shared origin so the session
cookie stays first-party: Caddy (or the Vite dev proxy) sends backend
paths and every non-GET request to Axum and page requests to SvelteKit.

## What's inside

- Every abyssal module on the contract market, searchable by attribute
  ranges, source type and mutaplasmid node, with roll-quality bars.
- A workbench to compare modules side by side, share as a link or keep as
  a collection.
- A mutation calculator that gets derived attributes like DPS right, with
  probability and combination tables.
- A value estimator: random forests trained in-process from contract
  history, every model resident in memory.
- Character asset and contract sync from ESI, sell flows, offers,
  notifications, premium themes.
- Four languages (English, German, Chinese, Russian), documentation
  included.
- A public JSON API with the reference generated from the routes.

## Run it

The one-command stack:

```sh
cp .env.example .env            # fill in EVE_CLIENT_ID / EVE_CLIENT_SECRET
docker compose up --build       # http://localhost:5100
```

It starts Postgres, downloads and seeds the latest SDE when it changed,
and serves the app. For hot reloading, run the dev frontend instead of the
built one:

```sh
docker compose --profile dev up -d postgres api frontend-dev
```

Native development: `cargo run` (API on `BIND_ADDR`, default 127.0.0.1:3000)
and `npm run dev` in `frontend/` (its Vite proxy points at the API), with
Postgres from `docker compose up -d postgres`.

EVE SSO login needs an application from
[developers.eveonline.com](https://developers.eveonline.com) whose callback
URL is `EVE_CALLBACK_URL` (through the shared origin, never Axum's port).
Every other setting has a working default; see `.env.example`.

## Data

- `cargo run --bin sde_import` seeds the reference tables from the SDE
  (skipped when the seeded build is current; `SDE_FORCE=1` reseeds).
- The live market, character assets and contracts come from ESI through
  the scheduler jobs; the admin console at `/admin` shows and triggers
  them. The mail and structure jobs need an admin to log the service
  character in through `/eve/admin` once.
- `LEGACY_IMPORT_CONFIRM=1 cargo run --bin legacy_import` is the one-time
  bootstrap from the legacy MySQL database (`LEGACY_DATABASE_URL`); it
  wipes and replays the domain tables. The advertisement and gear
  creatives it references live in deployment storage (`assets/img/ads`,
  `assets/img/gear`), not in this repository.

## Tests

```sh
cargo test                       # unit tests plus the integration binary
cargo clippy --all-targets       # zero warnings
cd frontend && npm test          # vitest
```

The integration suites run against the `mutamarket_test` database, created
automatically on the same Postgres, single-threaded (see
`.cargo/config.toml`). One suite at a time: `cargo test <module>::`.

## Deploy

On a machine with Docker and the domain's DNS pointing at it:

```sh
git clone https://github.com/MutaMarket/MutaMarket.git mutamarket && cd mutamarket
deploy/setup.sh
```

The setup checks the domain, walks through the EVE application (it says
where to create it and which callback and scopes to register), the optional
integrations (EVE mail sender, Discord alerts and invites, Patreon premium
sync and tiers, account linking, partner links), writes `.env` and starts
the stack. Rerun it any time to change a value. Updates are a `git pull`
followed by

```sh
docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d --build
```

Caddy (`deploy/Caddyfile`) holds the domain's certificate and sends HSTS;
the API runs as a non-root user with the OpenGraph cache and synced
creatives on volumes; `GET /api/health` answers 200 while the database
does. Back the `postgres-data` volume up with
`docker compose exec postgres pg_dump -U mutamarket mutamarket`.

## Contributing

Issues and pull requests are welcome. CI runs rustfmt, clippy with
warnings denied, the Rust suites against Postgres, and the frontend
check, lint, format and tests; `main` only takes green pull requests.

## License

MIT, see `LICENSE`. EVE Online and the EVE logo are trademarks of CCP hf.;
the type icons are used under the EVE developer license.
