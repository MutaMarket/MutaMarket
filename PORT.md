# MutaMarket port specification

The living spec and status tracker for the Laravel → Rust (Leptos SSR +
Axum + Postgres) rewrite. The legacy app at `the legacy checkout`
is the authority; this document records **what every route returns**, **the
behaviour behind it**, and **how far the port has got**.

Status legend: **DONE** (implemented and tested in this repo, file cited) ·
**PARTIAL** (some of it exists; the gap is named) · **STUB** (route
registered, returns a placeholder/redirect only) · **MISSING** (no route).

Keep this file honest: update the status marker in the same commit that
changes the behaviour.

---

## 1. Route inventory and response contracts

### 1.1 Public pages (Inertia in legacy, Leptos SSR here)

| Method · Path | Legacy controller@action | Response contract | Status | Impl |
|---|---|---|---|---|
| GET `/` | ModuleController@index | Modules browser page; for-sale modules (`hasLatestContract` OR public assets), `withDefaultRelations`, `simplePaginate(40)`; props: `modules`, `search`, `available_types`, `stats` | **PARTIAL** — grid + filters + market stats strip render; no pagination links/meta, no `available_types`, no list/table modes | `src/pages/modules_page.rs` |
| GET `/modules/add` | ModuleController@create | "Add module" page (paste PYFA / showinfo link) | **STUB** — placeholder page | `src/app.rs` |
| GET `/modules/{module}` | ModuleController@show | Single module detail page; module with all relations, `source_type_comparisons`, probability data | **PARTIAL** — card + basic detail; no source-type comparison table, no probability/estimator sidebar | `src/pages/modules_page.rs` |
| GET `/modules/{query?}` | ModuleController@index | Same as `/` with filter segments | **PARTIAL** — as `/` | `src/pages/modules_page.rs` |
| GET `/all-modules/{query?}` | AllModulesController@index | Browser incl. modules with no live contract | **PARTIAL** — renders; pagination/stats gaps as `/` | `src/pages/modules_page.rs` (`include_unlisted`) |
| GET `/characters` | CharacterController@index | `Characters/ShowAllCharactersPage`; characters with public ownerships, premium-first, `paginate(32)`; props `characters`, `search` | **PARTIAL** — index renders; no pagination, no search box UI | `src/pages/social_pages.rs` |
| GET `/characters/{character}/{query?}` | CharacterController@show | `Characters/ShowCharacterPage`; props `character`, `modules`, `search`, `stats`, `available_types`, `locations` (owner only) | **PARTIAL** — character + owned modules; no stats, available_types, locations, `created` mode toggle | `src/pages/social_pages.rs` |
| GET `/collections` | CollectionController@index | `Collections/ShowAllCollectionsPage`; `public_collections` (paginate 12) + `personal_collections` | **PARTIAL** — public index; no personal list, no pagination | `src/pages/social_pages.rs` |
| GET `/collections/{collection}/{query?}` | CollectionController@show | `Collections/ShowCollectionPage`; `collection`, `modules`, `search`, `available_types`, `stats`, `locations`; policy: private = owner only | **PARTIAL** — modules render, 403 policy works; no stats/available_types/locations/auto-sync UI | `src/pages/social_pages.rs` |
| GET `/calculator/{query?}` | CalculatorController@index | Roll calculator page (pick type + mutaplasmid, live roll math) | **STUB** — placeholder; mutation math is ported and reusable | `src/app.rs` |
| GET `/statistics/{query?}` | StatisticsController@index | Statistics page (market-wide roll/price stats) | **STUB** — placeholder | `src/app.rs` |
| GET `/premium` | PremiumController@index | Premium info / purchase page | **STUB** — placeholder | `src/app.rs` |
| GET `/omega-calculator` | OmegaCalculatorController@index | Omega vs Alpha value calculator | **STUB** — placeholder | `src/app.rs` |
| GET `/documentation/{page?}` | DocumentationController@show | Docs page; markdown rendered; 404 unknown, 503 on load failure | **DONE** | `src/pages/documentation.rs`, `src/docs.rs` |
| GET `/donations` | DonationsController@index | Donations / supporters page | **STUB** — placeholder | `src/app.rs` |
| GET `/moderator/contracts/{query?}` | ModeratorContractController@index | Contract-review queue (moderator) | **STUB** — placeholder | `src/app.rs` |
| GET `/workbench/{modules}` | WorkbenchController@index | Comparison workbench for a set of modules | **STUB** — placeholder | `src/app.rs` |
| GET `/login` | AuthController@login | Login page with EVE SSO entry | **DONE** | `src/pages/login.rs` |
| GET `/about` → `/documentation/about` (301) | — | Redirect | **DONE** | `src/server/mod.rs` |
| GET `/help` → `/documentation` (301) | — | Redirect | **DONE** | `src/server/mod.rs` |
| GET `/{query?}` | NotFoundController@index | Catch-all 404 page | **DONE** | leptos fallback |

### 1.2 Authenticated pages

| Method · Path | Controller@action | Response | Status | Impl |
|---|---|---|---|---|
| GET `/personal/modules/{query?}` | PersonalModuleController@index | Owned modules (assets ∪ contract items), `simplePaginate(40)`, `asset_import` prop | **PARTIAL** — grid + live import panel over `/ws`; no filter sidebar, no pagination | `src/pages/personal_modules.rs` |
| GET `/sell/modules/{query?}` | SellController@index | Sell listings management page | **STUB** — guest redirect only | `src/server/mod.rs` |
| GET `/locations` | LocationController@index | Owner's locations index | **STUB** — guest redirect | `src/server/mod.rs` |
| GET `/locations/{location}/{query?}` | LocationController@show | Modules at a location; `LocationResource` tree | **STUB** — guest redirect | `src/server/mod.rs` |
| GET `/historic-sales/{query?}` | HistoricSaleController@index (premium) | Historic sale browser | **STUB** — guest redirect; needs historic contracts | `src/server/mod.rs` |
| GET `/personal/contracts` | ContractController@index | User's ESI contracts | **STUB** — guest redirect; ingestion exists (`src/contracts/character.rs`), no page | `src/server/mod.rs` |
| GET `/personal/stats` | StatsController@index | Personal stats dashboard | **STUB** — guest redirect | `src/server/mod.rs` |
| GET `/settings` | SettingController@index | Settings page (notify character, scopes, links) | **STUB** — guest redirect | `src/server/mod.rs` |
| GET `/offers` | OfferController@index | Offers inbox | **STUB** — guest redirect; needs messages | `src/server/mod.rs` |
| GET `/offers/{offer}` | OfferController@show | Single offer thread | **STUB** — guest redirect | `src/server/mod.rs` |

### 1.3 Authenticated actions (POST/PUT/DELETE)

| Route | Controller@action | Behaviour | Status | Impl |
|---|---|---|---|---|
| PUT `/display` | DisplayController@update | Save 3 display cookies (public), redirect back | **DONE** | `src/server/display.rs` |
| POST `/modules` | ModuleController@store | Submit a module by showinfo link / item id | **STUB** — `not_implemented` (501) | `src/server/mod.rs` |
| PUT `/characters/{character}` | CharacterController@update | Edit own character bio (max 5000) | **DONE** | `src/server/social.rs` |
| DELETE `/auth/character/{character}` | UserCharacterController@destroy | Unlink character (not last, owner only), reassign active | **DONE** | `src/server/auth.rs` |
| PUT `/auth/character/{character}` | UserCharacterController@update | Set active character (owner only) | **DONE** | `src/server/auth.rs` |
| POST `/collections` | CollectionController@store | Create collection | **DONE** | `src/server/social.rs` |
| POST `/collections/modules` | CollectionController@storeAndAddModules | Create + fill collection | **DONE** | `src/server/social.rs` |
| PUT `/collections/{collection}` | CollectionController@update | Update collection (owner) | **DONE** | `src/server/social.rs` |
| DELETE `/collections/{collection}` | CollectionController@destroy | Delete collection (owner) | **DONE** | `src/server/social.rs` |
| POST `/collection-modules` | CollectionModuleController@store | Add module to collection | **DONE** | `src/server/social.rs` |
| PUT `/collection-modules/{id}` | CollectionModuleController@update | Update note | **DONE** | `src/server/social.rs` |
| DELETE `/collection-modules/all` | CollectionModuleController@destroyAll | Clear a collection | **DONE** | `src/server/social.rs` |
| DELETE `/collection-modules/{id}` | CollectionModuleController@destroy | Remove one | **DONE** | `src/server/social.rs` |
| POST `/estimate/{module}` | EstimatorController@update | Re-run estimate synchronously, redirect back | **DONE** | `src/server/estimate.rs` |
| POST `/logout` | AuthController@delete | Destroy session | **DONE** | `src/server/auth.rs` |
| PUT `/discord` `/twitch` `/patreon` | *Controller@update | Toggle public visibility of linked account | **STUB** — guest redirect; needs settings + `*_is_public` columns | `src/server/mod.rs` |
| POST `/public-assets` · DELETE `/public-assets/{id}` | PublicAssetController | Publish / unpublish an owned asset subtree; populates ownerships | **DONE** | `src/server/personal.rs`, `src/assets/public.rs` |
| POST `/settings` · PUT `/settings` | SettingController | Save settings | **STUB** — guest redirect | `src/server/mod.rs` |
| POST `/offers` · DELETE `/offers/{id}` | OfferController | Make / withdraw an offer | **STUB** — guest redirect | `src/server/mod.rs` |
| POST `/messages` | MessageController@store | Send message in an offer thread | **STUB** — guest redirect | `src/server/mod.rs` |
| POST `/notes` · POST `/collection-notes` | NoteController / CollectionNoteController | Private notes | **STUB** — guest redirect | `src/server/mod.rs` |
| POST `/module-pricing` | ModulePricingController@store | Set a module's asking price | **STUB** — guest redirect | `src/server/mod.rs` |
| POST `/bookmarks` · PUT/DELETE `/bookmarks/{id}` | BookmarkController | Bookmark modules | **STUB** — guest redirect | `src/server/mod.rs` |
| POST `/blocked-users` | BlockedUserController@store | Block a user | **STUB** — guest redirect | `src/server/mod.rs` |
| POST `/ui/contract` | UIController@openContract | Open in-game contract window via ESI UI scope | **STUB** — guest redirect | `src/server/mod.rs` |
| POST `/personal/contracts` | ContractController@store | Trigger personal contract fetch | **STUB** — guest redirect; ingestion exists | `src/server/mod.rs` |
| Collection auto-sync (`/collections/{c}/auto-sync[...]`), collection-locations, location-collections | Collection*Controller | Location-based collection auto-sync | **STUB** — guest redirect | `src/server/mod.rs` |
| Workbench (`/workbench/{m}`, `/workbench-modules[...]`, `/workbench-collections`) | Workbench*Controller | Comparison workbench CRUD | **STUB** — guest redirect | `src/server/mod.rs` |
| PUT/DELETE `/raffle/{item}` | RaffleController | User raffle entry actions | **STUB** — guest redirect | `src/server/mod.rs` |

### 1.4 Admin / moderator (admin middleware)

| Route | Controller | Status |
|---|---|---|
| PUT `/historic-contracts/{id}` | HistoricContractsController@update | **MISSING** — route not registered |
| GET/POST `/raffles` | Admin\RaffleController | **STUB** — `/raffles` guest redirect; no admin gate |
| GET/POST/PATCH/DELETE `/advertisements[...]` | Admin\AdvertisementController | **STUB** — `/advertisements` guest redirect |
| POST `/moderator/contracts/{historicContract}` | ModeratorContractController@store | **MISSING** |

### 1.5 OpenGraph images

| Route | Legacy | Status | Impl |
|---|---|---|---|
| GET `/og/module/{module}` | Rendered PNG card | **PARTIAL** — 404 unknown, else redirect to EVE type icon (documented divergence; no bespoke card renderer) | `src/server/social.rs` |
| GET `/og/type/{type}` | PNG card | **PARTIAL** — as above | `src/server/social.rs` |
| GET `/og/character/{character}` | PNG card | **PARTIAL** — redirect to portrait | `src/server/social.rs` |
| GET `/og/collection/{collection}` | PNG card | **PARTIAL** — redirect to logo | `src/server/social.rs` |

### 1.6 OAuth / SSO

| Route | Status | Impl |
|---|---|---|
| GET `/eve` (+`?add_to_account`, `?scopes`, `?without_scopes`) | **DONE** | `src/server/auth.rs` |
| GET `/eve/corporation` | **DONE** | `src/server/auth.rs` |
| GET `/eve/admin` | **DONE** (scope set shrunk: mail/wallet retired) | `src/server/auth.rs` |
| GET `/eve/callback` | **DONE** (owner-hash resolution, add-to-account, orphan cleanup) | `src/server/auth.rs` |
| GET `/twitch` `/twitch/callback` (+ discord, patreon) | **DONE** — link flows | `src/server/linked.rs` |

### 1.7 JSON API

| Route | Response | Status | Impl |
|---|---|---|---|
| GET `/api/modules/{module}` | `{data: ModuleResource}` | **DONE** (key-set parity tested) | `src/server/api.rs` |
| GET `/api/modules/{query?}` | `{data:[…], links, meta}` cursor-paginated (100/page); 404 no type | **DONE** — cursor pagination via offset (opaque-cursor divergence) | `src/server/api.rs` |
| POST `/api/modules` | Submit module; Laravel-shaped 422 | **DONE** | `src/server/api.rs` |
| GET `/api/estimator-statistics` | Bare array of statistic rows | **DONE** | `src/server/api.rs` |
| GET `/api/abyssal-type-statistics` | Bare array; `meta_level` absent (whenHas quirk) | **DONE** | `src/server/api.rs` |

---

## 2. Resource catalog (exact key sets)

Conditional keys: `whenLoaded` = present-null when the relation is loaded
but empty, absent when unloaded; `whenHas` = present only when the model
attribute is set; `whenCounted` = present when the count was eager-loaded.

- **ModuleResource** (`ModuleResource.php`): `id`, `type`(ModuleType), `creator`(Character), `mutated_attributes`([MutatedAttribute]), `source_type`(Type), `mutaplasmid`(Mutaplasmid), `contract`(Contract), `estimated_value`, `estimated_value_updated_at`, `asset`(Asset), `source_type_comparisons`([SourceTypeComparison]), `training_module`(TrainingModule), `collections`([{id,collection_module_id,name,slug}]), `collection_note`(CollectionNote), `public_asset`(PublicAsset), `latest_offer`({id,sender,receiver,left_by_sender_at,left_by_receiver_at} — only if offer.receiver = asset owner), `note`(Note), `slug`, `average_fraction`. **Ported subset** (`src/modules/view.rs`): id, type, creator, mutated_attributes, source_type, mutaplasmid, contract, estimated_value(+updated_at), public_asset, slug, average_fraction. **Missing keys**: source_type_comparisons, training_module, collections, collection_note, latest_offer, note, asset (asset is populated only on the personal page via a side query, not in the resource).
- **MutatedAttributeResource**: `id`(=attribute_id), `name`, `display_name`, `value`, `base_value`, `fraction`, `fraction_type`, `fraction_absolute`, `bar`, `is_derived`, `unit`(Unit), `is_virtual`. **DONE** — `src/modules/view.rs::ModuleAttributeView`.
- **ContractResource**: `id`, `type`, `price`(=unified_price), `asking_for_items`, `plex_count`, `non_abyssal_modules_count`, `abyssal_modules_count`, `issuer`(Character), `status`(whenHas), `modules`(whenLoaded), `types`(whenLoaded), `is_private`(whenHas availability), `acceptor`(whenHas), `acceptor_type`(whenHas), `date_issued`, `date_expired`, `date_accepted`(whenHas), `ignore_for_training`(admin+whenHas). **Ported subset** (`ContractRef`): id, type, price, asking_for_items, plex_count, non_abyssal_modules_count, abyssal_modules_count, issuer, date_issued, date_expired. **Missing**: status, modules, types, is_private, acceptor(+type), date_accepted, ignore_for_training (all personal/historic-contract keys).
- **CharacterResource**: `id`, `slug`, `name`, `description`, `has_premium`, `modules_count`(whenCounted), `corporation_id`, `discord`/`twitch`/`patreon`(whenLoaded user + public flag), `modules_created_count`(whenHas), `rank_number`(whenHas). **Ported subset** (`CharacterRef`/`CharacterView`): id, slug, name, description, has_premium, corporation_id, modules_count. **Missing**: discord/twitch/patreon, modules_created_count, rank_number.
- **AuthenticatedUserResource**: `id`, `name`, `is_admin`, `active_character`(AuthenticatedCharacter), `characters`([AuthenticatedCharacter]), `character_to_notify`, `discord`/`twitch`/`patreon`(when linked). **PARTIAL** — the character menu server fn (`src/pages/character_menu.rs`) returns id/name/corp/has_asset_token/active; no is_admin, character_to_notify, linked-account details.
- **AuthenticatedCharacterResource**: `id`, `user_id`, `corporation_id`, `has_corporation_token`, `name`, `premium_paid_total`, `premium_paid_until`, `slug`, `has_asset_token`, `has_premium`. **PARTIAL** — has_asset_token + active are surfaced; missing has_corporation_token, premium fields.
- **ModuleTypeResource / TypeResource**: `{id, name}` (+ meta_group, meta_group_id, published on TypeResource). **DONE** — `TypeRef`/`SourceTypeRef`.
- **AssetResource / LocationResource** (`Locations/`): parent_name, parent_type_id, parent_slug, station{id,name,type_id,slug}, location_id, location_type, location_flag, location_index, corporation_id, owner, + LocationResource adds type_id/type/asset_id/public_asset_id/name/modules_count/item_id/slug. **PARTIAL** — `AssetLocationView` (`src/modules/view.rs`) covers the AssetResource footer subset; the full LocationResource (for `/locations` pages) is not built.
- **CollectionResource / UserCollectionResource** (`Collections/`): id, slug, name, description, visibility, character, types, modules_count, + auto-sync/location fields. **PARTIAL** — `CollectionCardData` covers listing subset.
- **AbyssalTypeStatisticResource**: id, type_id, attribute_id, high_is_good, is_virtual, best, worst, is_derived, attribute{…}, type{…} (meta_level absent). **DONE**.
- **EstimatorQueryResource**: flat `{feature: value}` for the AI server. **DONE** — `src/estimator/`.
- **Not yet needed** (no ported consumer): OfferResource, MessageResource, BookmarkResource, NoteResource, CollectionNoteResource, PricingResource, ProbabilityResource, SourceTypeComparisonResource, TrainingModuleResource, RaffleResource/RaffleWinResource, DonationResource, Discord/Twitch/PatreonDetailsResource, WorkbenchModuleResource, RecursiveLocationResource/StationResource.

---

## 3. Feature map (behaviour & background work)

### 3.1 Scheduled jobs (legacy `routes/console.php`)

| Command | Cadence | Ported? | Impl |
|---|---|---|---|
| GetPublicContractsCommand | 30 min | **DONE** | `src/scheduler.rs`, `src/contracts/mod.rs` |
| GetPublicContractBidsCommand | 5 min | **DONE** | `src/contracts/mod.rs::sync_auction_bids` |
| GetCharacterContractsCommand | 5 min | **DONE** | `src/contracts/character.rs` |
| GetCharacterNamesCommand | 1 min | **DONE** (bisect on 404) | `src/characters/mod.rs` |
| GetMarketHistoriesCommand | daily | **DONE** (PLEX only) | `src/contracts/mod.rs::sync_plex_market_history` |
| GetPublicStructuresCommand | daily | **DONE** | `src/structures/mod.rs` |
| GetCharacterAssetsCommand | 5 min | **DONE** (nameable-type prefilter + bisect) | `src/assets/mod.rs` |
| FailStaleAssetImportsCommand | 1 min | **DONE** | `src/assets/mod.rs::fail_stale_asset_imports` |
| EstimateValuesCommand | 5 min | **DONE** | `src/scheduler.rs`, `src/estimator/` |
| GetAlliancesCommand | daily | **MISSING** — alliances not modelled |
| NotifyUsersCommand | 1 min | **MISSING** — offers/messages notifications |
| GetMailsCommand | 30 s | **MISSING** — mail scope retired; needs redesign |
| GetWalletJournalCommand | 1 min | **MISSING** — wallet scope retired; donation tracking |
| DrawRaffleWinnerCommand | hourly | **MISSING** — raffles |
| TrainEstimatorsCommand | Mondays | **MISSING** — needs historic contracts + python sidecar |
| SearchTrainingModulesCommand | hourly | **MISSING** — training data selection |
| CheckAdminScopesCommand | hourly | **MISSING** |
| CreateSitemapCommand | daily 11:00 | **MISSING** |
| SnapshotCommand | 5 min | **MISSING** — market snapshots for statistics |
| RemoveExpiredPremiumCommand | 5 min | **MISSING** — premium lifecycle |
| SyncAdvertisementStatesCommand | 5 min | **MISSING** — ads |
| GetPatreonSubscribers | 10 min | **MISSING** — premium via Patreon |
| Prune*/Terminate/ClearOGCache | various | **MISSING** — housekeeping |

### 3.2 Cross-cutting mechanisms

- **Sessions / active character**: session cookie + `sessions.active_character_id`. **DONE** (`src/auth/session.rs`).
- **Display cookies** (display, attribute_bar_mode, show_attribute_scores). **DONE**.
- **ESI token refresh** (5-min buffer, rotate, delete-on-reject). **DONE** (`src/auth/tokens.rs`).
- **Broadcasts / websockets**: legacy uses Echo/Reverb private channel `Users.{id}` for messages/offers; we added `/ws` for live asset-import progress (an upgrade over legacy's 2-s Inertia poll). **PARTIAL** — only `AssetImportUpdated`; messages/offers events not built. (`src/server/ws.rs`)
- **Tracing**: `RUST_LOG`-configurable structured logs; ESI failures log URL+status. **DONE** (`src/main.rs`, `src/esi/mod.rs`).
- **Ownership union** (assets ∪ contract items → owned modules): computed in SQL; legacy uses trigger-maintained `module_ownerships` / `public_module_ownerships`. **PARTIAL** — `public_module_ownerships` is populated by the publish flow (`/characters` index now fills as users publish); the trigger-maintained contract-item half is still derived on read.
- **Notifications / flash toasts**: legacy `->notify(...)`. **MISSING** — no flash mechanism; redirects carry no toast.
- **Premium lifecycle** (Patreon/ISK, `premium_paid_until`): column exists, no ingestion. **MISSING**.

---

## 4. Outstanding backlog (prioritised)

Milestones ordered by dependency and value. `[ ]` = not started, `[~]` =
partial.

### M1 — Finish the module browser (no new external deps)
- [~] Pagination links/meta on the browser pages (API already paginates)
- [ ] List and table view modes (display cookie already switches; only `grid` implemented)
- [x] `stats` on `/` and `/all-modules` (ModulesStats strip) — `src/modules/stats.rs`
- [ ] Source-type comparison table + probability sidebar on module detail (`source_type_comparisons`, ProbabilityResource)

### M2 — Sell / public assets flow (unlocks `/characters`, `/sell`)
- [x] `POST/DELETE /public-assets` — publish/unpublish owned asset subtree (`src/assets/public.rs`)
- [x] Populate `public_module_ownerships` when an asset is published
- [ ] `/sell/modules` page + `POST /module-pricing`
- [ ] Character page `stats`, `available_types`, `created` mode
- Depends on: nothing new (assets ingestion is DONE)

### M3 — Locations
- [ ] Full `LocationResource` / `RecursiveLocationResource` tree
- [ ] `/locations` + `/locations/{location}` pages
- [ ] Collection auto-sync (`collection-locations`, `location-collections`, auto-sync CRUD)

### M4 — Offers & messages (social)
- [ ] `offers` table + `OfferResource`; `/offers`, `/offers/{id}` pages
- [ ] `POST/DELETE /offers`, `POST /messages`, `MessageResource`
- [ ] `Users.{id}` websocket events for new offers/messages
- [ ] `NotifyUsersCommand` equivalent
- [ ] `latest_offer` key on ModuleResource
- Depends on: M2 (offers attach to public assets)

### M5 — Historic contracts & estimator training
- [ ] Historic contracts ingestion (moved-off-feed contracts → training data)
- [ ] `HistoricSaleController` page (premium-gated)
- [ ] Moderator contract review (`/moderator/contracts`, `historic-contracts` update)
- [ ] `TrainEstimatorsCommand` + `SearchTrainingModulesCommand` (python sidecar)
- [ ] `training_module` key on ModuleResource

### M6 — Toolbox pages (self-contained, math already ported)
- [ ] `/calculator` — roll calculator (reuse `src/mutation/`)
- [ ] `/omega-calculator`
- [ ] `/statistics` + `SnapshotCommand` market snapshots
- [ ] `/modules/add` + `POST /modules` store (link/PYFA parsing exists in `src/modules/link.rs`)

### M7 — Collections extras
- [ ] Collection `stats`, `available_types`, personal collections list, pagination
- [ ] Notes (`/notes`, `/collection-notes`), bookmarks
- [ ] Workbench (comparison) CRUD + page

### M8 — Settings, premium, misc
- [ ] `/settings` page + `POST/PUT /settings` (notify character, scope management, `discord/twitch/patreon` public toggles)
- [ ] Premium: Patreon subscriber sync, `RemoveExpiredPremiumCommand`, `/premium` page, `/donations`
- [ ] Blocked users
- [ ] `POST /ui/contract` (ESI open-window)
- [ ] Alliances model + `GetAlliancesCommand`

### M9 — Admin
- [ ] Admin gate middleware in the router
- [ ] Raffles (`/raffles`, draw command, user raffle actions)
- [ ] Advertisements CRUD + `SyncAdvertisementStatesCommand`
- [ ] `CheckAdminScopesCommand`, sitemap, OG PNG renderer + cache, housekeeping prunes

### Cannot port as-is (retired ESI scopes)
- EVE-mail submissions (`GetMailsCommand`) — mail scope retired
- Wallet-based donation tracking (`GetWalletJournalCommand`) — wallet scope retired
- Both need a redesigned mechanism when reached.

---

## 5. Known deliberate divergences

- axum 303/307 redirects vs Laravel 302 (project-wide).
- OAuth state + add-to-account in HttpOnly cookies vs Laravel server session.
- OG endpoints redirect to EVE images instead of rendering PNG cards.
- API cursor encodes an offset, not a keyset pointer (opaque to clients).
- Station names composed natively from the SDE celestial chain (legacy dumped a MySQL table); asset names filtered to Ships/Containers market groups with a bisecting fallback.
- `mutaplasmid_type_statistics` / `abyssal_type_statistics` computed at import, not shipped by the SDE.
- Live progress pushed over `/ws` instead of legacy's 2-second Inertia polling.
- ESI structure scope uses the renamed `esi-structures.read_character.v1`.
- `PUT /display` answers 204 + `Set-Cookie` instead of the legacy redirect-back (only fetch() clients call it since the SvelteKit pivot).
- The `/api/personal/*` page-data endpoints answer guests with 401 `{"message":"Unauthenticated."}` instead of the page routes' login redirect.
