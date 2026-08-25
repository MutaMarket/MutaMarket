# Spec: the module show page and the module card

The porting spec for the legacy `ShowModulePage.vue` and the grid module
card, transcribed component by component from
`the legacy checkout/resources/js` (the authority — reread the
Vue file before porting a piece; this document is the map, not the
territory). Current SvelteKit state for comparison:
`frontend/src/lib/components/module-card.svelte` and
`frontend/src/routes/modules/[...query]` carry a reduced card and detail
view; everything marked **[gap]** below is not ported yet.

Theme note: layout, structure, behavior and wording port one-to-one;
colors/radius/typography follow our shadcn theme (a documented
exception). Legacy `text-white` maps to our `text-foreground`. The HUD
utilities used here already exist in `frontend/src/routes/layout.css`
(`hud-panel`, `hud-label`) — **[gap]** `hud-readout` (mono, tabular-nums,
letter-spacing 0.05em) and `hud-scan` (one-shot diagonal glow sweep on
mount, reduced-motion guarded) must be ported from the legacy
`resources/css/app.css` (lines ~169–210).

---

## 1. Page layout — `Pages/Modules/ShowModulePage.vue`

Inertia props (per-page data the API must serve; our
`GET /api/modules/{slug}` covers only `module` today):

| Prop | Type | Purpose | Status |
|---|---|---|---|
| `module` | ModuleResource | the module, default relations | ported |
| `attribute_bar_mode`, `show_attribute_scores` | display cookies | bar/score rendering | ported (cookies) |
| `estimator_statistic` | EstimatorStatistic | the type's model quality sheet | **[gap]** (endpoint exists: `/api/estimator-statistics` serves all; page wants the module's type's row) |
| `market_histories` | map type_id → {average,…} | source-type prices | **[gap]** |
| `historic_contracts` | HistoricContractResource[] | contract-history tab | **[gap]** (historic contracts unported) |
| `abyssal_type_statistics` | rows for the type | roll extremes | endpoint exists |
| `similar_modules?` | ModuleResource[] (deferred, premium) | similar-sold tab | **[gap]** |

Structure (12-col grid, `gap-4`):

```
<Main class="grid grid-cols-12 gap-4">
  col-span-full md:col-span-4   → Grid/Module.vue        (the card, §3)
  col-span-full md:col-span-8   → Show/ModuleHero.vue    (§2)
  col-span-full                 → InlineAd + Show/ModuleTabs.vue (§4)
```

Meta: `<Headers>` sets title "{creator}'s {type}", a description listing
every mutated attribute as "{display_name}: {formatted}" plus the
estimated value line (with a "(low confidence)" suffix when the
statistic's r² < 0.1), and the OG image `/og/module/{id}.png` sized
350 × (72 + 50·non-virtual-attribute-count). **[gap]** our OG endpoint
redirects to the type icon instead of rendering the card PNG (documented
divergence in PORT.md).

## 2. The hero — `Show/ModuleHero.vue`

`div.hud-panel.relative.flex.flex-col` with an absolutely-inset
`hud-scan` overlay (aria-hidden), containing top to bottom:

1. **CreatorDetails** (`Modules/CreatorDetails.vue`), bottom-bordered:
   a Link to the creator's character page, `flex items-center gap-4 p-4`;
   10×10 rounded character portrait (CharacterIcon); two lines — muted
   sm "Created by" label, then the name `font-medium`, `text-gold` when
   `creator.has_premium`.
2. **EstimatorStatistics** (`Statistics/Estimators/EstimatorStatistics.vue`, §2.1).
3. **ModuleToolbar** (§2.2), top-bordered.

### 2.1 EstimatorStatistics

Renders only when `estimator_statistic.r2 !== null && mae !== null`,
else **MissingData** (`p-2`; sm "AI value estimation" heading line,
`font-medium` heading, `text-2xs` body naming `data_count` — exact copy in
`stats.estimators.missingData*` locale keys).

When present, `flex flex-col`:

- **EstimatedValue** (`Statistics/Estimators/EstimatedValue.vue`),
  `flex grow flex-col gap-1.5 p-4`:
  - `h2.hud-label` "AI value prediction" + an `Info` icon (size-3) whose
    tooltip carries the AI explainer copy.
  - A `<button>` (click copies the value very-compact to the clipboard,
    success toast): the value in
    `hud-readout text-primary text-2xl [text-shadow:0_0_18px_var(--glow)]`
    formatted `toIskCompact` ("142 million ISK"), plus
    `±{nmae.toFixed(0)}%` in `hud-readout text-muted-foreground ml-2`.
  - Muted xs "evaluated {time} ago" line; the distance string re-renders
    every second (`useIntervalFn` 1000 ms, `formatDistanceToNowStrict`).
- **Stat grid** `grid grid-cols-2 sm:grid-cols-3 border-t`, cells
  `flex flex-col gap-1 p-4` with hairline dividers
  (`border-l`/`border-t` per position):
  1. *Confidence*: hud-label; a word (`hud-readout text-lg uppercase`)
     from the score ladder — ≥4.5 "Very high" (text-positive), ≥3.5
     "High" (positive), ≥2.5 "Moderate" (text-primary), ≥1.5 "Low"
     (negative), else "Very low" (negative); sub-line `R² {r2.toFixed(2)}`.
     Confidence stars = r² clamped 0..1 mapped to 1–5 (full stars = 1s,
     half = 2, see `useEstimatorStatistics.getStarsArray`).
  2. *Bias score* (only when `data_statistics` is a keyed object): same
     word ladder over the bias score = normalized Shannon entropy of the
     per-source-type sample counts × a total-count factor (0 below 10
     samples, 1 above 100, linear between); sub-line "{n} samples"
     (localized plural, `tabular-nums`).
  3. *Avg. error*: `±{mae}` very-compact (`hud-readout text-lg`), hint
     sub-line.
  4. *Last trained*: "{time} ago", ticking like the evaluated line.
  5. *Training data* (spans rows on sm, `sm:col-start-3 sm:row-span-2`):
     hud-label; a two-column `text-xs` grid of source-type name (muted,
     truncate) → count (right, tabular-nums, `text-negative` when < 10);
     footer Link "View historic sales →" (`text-primary`, MoveRight
     icon) to `/historic-sales/type/{type_id}`.

### 2.2 ModuleToolbar + ToolbarButton

`TooltipProvider` (300 ms delay) around a
`flex flex-wrap items-center gap-1 border-t px-3 py-2` row. Every button
is **ToolbarButton**: ghost Button `gap-1 px-2` with a size-4 lucide icon,
optional size-3 60%-opacity ChevronDown when it opens a menu, and a
tooltip carrying the label (or the `disabled_reason` when disabled —
wrap in a span so disabled buttons still show the tooltip).

Order, with `mx-1 h-5 w-px bg-border` dividers between groups:

1. `Search` icon — "Search this type" → navigates to
   `QueryBuilder.make({type_id})` (our `/modules/type/{id}`).
2. `GitCompareArrows` + chevron — DropdownMenu → **SearchSimilarMenu** (§5).
3. `TrendingDown` + chevron — DropdownMenu → **SearchCheapestMenu** (§5).
4. `History` + chevron — DropdownMenu → **SearchHistoricMenu** (§5).
   — divider —
5. `FileCode2` "Pyfa" → copy Pyfa export (§6).
6. `Link` "Copy item link" → copy `<url=showinfo:...>` (§6).
7. `FileSignature` "Copy contract link" — disabled without
   `module.contract` (reason: "No active contract").
8. `ExternalLink` "Open contract in game" — disabled without contract;
   POSTs `/ui/contract` with the contract id.
   — divider —
9. `Share2` "Share module" → `navigator.share` with title
   "{creator}'s {type}" and the page URL; fallback (no share API or
   non-abort error): copy the page link.
10. `Ellipsis` "More" — DropdownMenu (align end): "Copy image link"
    (Image icon; §6 `useImage`) and "Download image" (`<a download>` of
    the OG PNG).

## 3. The card — `Module/Grid/Module.vue` (recursive)

Root: `ContextMenu` wrapping a `ContextMenuTrigger as="div"` with
`class="border-border grid rounded-lg border *:first:rounded-t-lg
*:last:rounded-b-lg"` and `style="gridRow: span {row_span}"`; right-click
anywhere opens **ModuleContextMenu** (§5). The masonry `row_span` =
1 (header) + 1 (location row) + visual-attribute count
+ note? + collection_note? + asking_price? — where *visual attributes* =
`!isApproximatelyEqual(value, 0) && !is_virtual`, and the note /
collection-note / asking-price rows count when their content exists OR
their editor is open (`useNote` / `useCollectionNote` /
`useAskingPrice` `is_editing`).

Children in order:

1. **Header** (§3.1)
2. **Attribute** × visual attributes (§3.2)
3. Exactly one "location row" (first match wins):
   `Training` if `module.training_module` → `Contract` if
   `module.contract` → `Asset` if `module.asset` → `PublicAsset` if
   `module.public_asset` → `EstimatedValue` (fallback). (§3.3)
4. **Note**, **CollectionNote**, **AskingPrice** — each self-hides (§3.4).

Current port status: header/attributes/bars ported (minus icons and
menus); location rows: only a reduced Asset variant and a static
"Est. value: N/A" footer. Everything else **[gap]**.

### 3.1 Header — `Grid/Header.vue`

`div` `bg-card-1 relative grid h-[50px] grid-cols-[36px_1fr_auto]
content-center items-center gap-x-2 border-b-2 p-2` with
`data-meta-group` driving the accent border:
t1→`border-b-gray-500`, t2→orange-500, storyline→green-300,
faction→green-500, officer→purple-500, deadspace→blue-500 (meta group
from `source_type.meta_group_id`, default t1). Contents:

- **AbyssalIcon** `row-span-2 size-8 rounded-lg` — image
  `/img/icons/{type_id}.png` with a Fallback component on error
  (**[gap]** we hotlink `images.evetech.net`; legacy serves local icons
  with fallback).
- Link to the module page, `truncate text-sm text-white`, text =
  `source_type.name`, containing the classic stretched-link
  `<span aria-hidden class="absolute inset-0"/>`.
- `mutaplasmid.name` in `mt-1 truncate text-xs text-muted-foreground`.
- Column 3, spanning both rows: a DropdownMenu — ghost icon Button
  (`text-2xl`) with a vertical-ellipsis icon → **ModuleDropdownMenu**
  (§5). **[gap]**

### 3.2 Attribute row — `Grid/Attribute.vue`

`bg-card-2 grid grid-cols-[36px_1fr_auto] content-center items-center
gap-x-2 px-2 py-1`:

- **AttributeIcon** `row-span-2 size-8` — `/img/icons/{attribute_id}.png`
  + fallback. **[gap]** (our row has no icon column; note the legacy grid
  template includes the 36px icon column — adding it changes alignment).
- `display_name` in `text-xs text-muted-foreground`.
- Value cluster `flex gap-1 text-sm text-white`: formatted value, then
  the difference span colored by `difference_type`:
  bar=1→`text-gold`, 2→`text-diamond`, −1→`text-brown`, else
  derived±→`text-positive-derived`/`text-negative-derived`,
  else ±→`text-positive`/`text-negative`. (Ported.)
- Column 3 both rows: **AttributeScore** when `show_attribute_scores` —
  `+/−{round(fraction_absolute·20 − 10)}` in `text-sm font-medium`,
  green-500 ≥ 0.66, yellow-500 ≥ 0.33, else red-500. (Ported.)
- Full-width `my-1` bar row by `attribute_bar_mode` (`default`/`type`/
  `absolute`; `none` renders nothing). (Ported — including the server-side
  `type_band` replacing the legacy client-side mutaplasmid-range math in
  `BarTypeNormalized`; the three bars' exact geometry is in
  `attribute-row.svelte` and matches `Bars/*.vue`.)

### 3.3 Location rows (h-[50px], `bg-card`)

All share `grid h-[50px] grid-cols-[36px_1fr] items-center px-2` with a
36px icon cell and a right-aligned two-line text block
(`grid text-right`; second line `text-muted-foreground text-sm
leading-4`). "Value line" below = "est. {toIskCompact}" or "No estimate".

- **Contract** (whole row Links to the module page): icon cell
  `text-amber-500` — ExchangeIcon for `item_exchange`, else AuctionIcon;
  when `contract.abyssal_modules_count > 1` a superscript
  `+{count−1}` xs badge right of the icon. Line 1 `toIskCompact(price)`;
  line 2 the value line. **[gap]**
- **EstimatedValue** (fallback; Links to module page): green-500 AIIcon;
  line 1 the value line; line 2 "Created by {creator.name}". **[gap]**
  (ours is a static "Est. value: N/A" div.)
- **Training** (sold training module; Links to module page): green-500
  AIIcon; line 1 `toIskCompact(sold_for)`; line 2
  "{value line} | sold {distanceToNowStrict(sold_at)} ago". **[gap]**
- **Asset** (owner's asset): a HoverCard whose trigger is a Link to
  `/locations/{parent_slug}` — `relative grid
  grid-cols-[36px_1fr_auto] items-center gap-2 p-2`; TypeIcon of
  `parent_type_id` (rounded-lg); text block `py-[3px] text-xs`: bold
  truncated `parent_name`, then muted
  "{LocationFlag label} | {est. value line (capitalized)}"; third column
  `pr-2 pl-4 font-medium` = `location_index + 1`. Hover opens
  **FindAssetTooltip**: a 400px `border bg-card p-8 text-sm` HoverCard
  with three sections — "Module location" ("belongs to **{owner}** in
  **{station}**", both linked), a numbered "How to find it" list (open
  container link → sort by type → search `{type.name}` with
  copy-to-clipboard button → count to item **index+1**), and a "Quick
  tip" ("resize inventory to 10 columns → row {⌊index/10⌋+1}, column
  {(index+1)%10 || 10}"). Partially ported (row without hover card,
  estimate segment, owner/station — needs `asset.owner`/`asset.station`
  in the payload **[gap]**).
- **PublicAsset** (a MutaMarket sell listing): owner's CharacterIcon
  `size-8 rounded-lg`; without `module.latest_offer` — a stretched-link
  `<button>` "{toIskCompact(price)}" (or "Make offer" when no price)
  opening the make-offer dialog, value line under; with an offer — Link
  "Go to offer" to `/offers/{id}`. **[gap]** (offers unported.)

### 3.4 Owner rows (h-[54px], border-t)

- **Note** / **CollectionNote** — display state: `bg-card grid h-[54px]
  grid-cols-[32px_1fr] items-center border-t px-2`; lime-500 NoteIcon in
  column 1, centered sm content text. Edit state (opened from the menus):
  same frame (`bg-gray-950` for collection note) with a full-bleed
  1-row Textarea (`pl-12`, placeholder "Add a note…") over the icon.
  Save/cancel semantics live in `useNote`/`useCollectionNote`
  (Enter saves via POST `/notes` / `/collection-notes`). **[gap]**
- **AskingPrice** — shown to the owner (`can_set_price`) when a price is
  set: amber CoinIcon, right block `toIskCompact(price)` bold +
  "Your asking price" muted sm. Edit state: number Input (right-aligned,
  `pl-12`) with a live very-compact preview in the middle column.
  **[gap]**

## 4. The tabs — `Show/ModuleTabs.vue`

`Tabs` in a `hud-panel block`; active tab persisted in the
`module_show_tab` cookie (values `market`/`contracts`/`similar`,
default `market` — cookie is already in the legacy display-settings set;
our `settingsFromCookies` doesn't carry it yet **[gap]**). TabsList
triggers with size-4 icons: ChartColumn "Source types", FileClock
"Contract history", PackageCheck "Similar sold".

1. **Source types** — `Tables/SourceTypes/TypesTable.vue`: a tanstack
   BaseTable comparing the module against every *published* input type of
   its mutaplasmid (legacy computes client-side from bundled static
   data; **we should compute server-side** from reference tables).
   Columns: type (TypeInfo cell: icon + name, compact when > 5
   attributes), meta-level (header = icon `633.png`, centered
   tabular-nums), one column per mutated attribute (header =
   TypeAttributeHeader with the attribute icon; cell = TypeValue showing
   the input type's value + the difference *from that type to this roll*,
   green/red by whether the roll beats it), and price (TypePrice from
   `market_histories[type_id].average`). Default order: meta-group rank
   (T1, T2, Storyline, Faction, Deadspace, Officer), then meta level,
   then name; headers sortable. **[gap]**
2. **Contract history** — BaseTable over `historic_contracts` plus the
   live contract appended as status `outstanding`, sorted id desc.
   Columns: id, issuer (CharacterDetails cell; name filter), issued
   at, expiry, multi-item badge (positive "Yes" when
   `non_abyssal + abyssal > 1`), status (outstanding first in sort),
   price (compact, right, lg), row actions dropdown (contract link
   copy etc.). **[gap]** (needs historic contracts.)
3. **Similar sold** — `Modules/SimilarModulesSold.vue`, premium-gated:
   - With premium: Inertia `Deferred` (skeleton: three animated stat
     stubs + 8 pulsing 280px card ghosts) resolving to a stat strip —
     Average / Lowest (`text-positive`, hover highlights the cheapest
     card with an emerald ring) / Highest (`text-negative`, rose ring) in
     `hud-label` + `hud-readout text-lg` cells with hairline dividers,
     plus an outline "View historic sales →" button — and a
     `repeat(auto-fill,minmax(270px,1fr))` grid of **Module cards**
     (recursion: the card embeds fully here, `training_module` rows
     showing sold prices).
   - Without premium: the same UI blurred (`blur-[14px]`, static teaser
     numbers + `useTeaserModules` fakes) under a centered upsell card
     (mono "Premium" eyebrow, title, three Check bullet points, full
     width "Upgrade" button linking `/premium`). **[gap]**

## 5. The menus (dropdown ≡ context)

**ModuleDropdownMenu** (header ⋯ button) and **ModuleContextMenu**
(right-click) are the *same menu* built from DropdownMenu* vs
ContextMenu* primitives — port once, parametrize the primitive set.
Content (w-60, side right for dropdown):

1. Three search submenus (SearchIcon triggers): *Search similar*,
   *Search cheapest*, *Search historic* — each hosting **SearchMenuForm**:
   - a NumberField "Variance" (min 1, default 1);
   - "Match attributes" list: every mutated attribute as a toggle row
     (attribute icon, name muted when disabled, primary Check when
     enabled) + a "Select all/Clear all" text button;
   - footer slot buttons: Similar → "Search modules for sale" (+
     "Search here" when the hosting page prefix ≠ `modules`, via
     provide/inject `prefix`); Cheapest/Historic → "Search".
   - Submit builds attribute bounds via `ModuleFinder`: for each enabled
     attribute, `step = (best − worst) · variance / 100` around the
     roll's value (`lower = value − step`, `upper = value + step`,
     best/worst from the type's abyssal statistics), then navigates:
     similar → `/{prefix}/type/{id}/attributes/...`; cheapest → same +
     sort price asc; historic → `/historic-sales/...`. On success the
     module is added to the workbench. **[gap]** (needs attribute-bounds
     URL segments in the TS query builder — the Rust grammar already
     parses `attributes/{name}/{lower}-{upper}`.)
2. Share group: "Share module" (page-link copy), "Open contract in
   game" (only with contract).
3. Copy group: Pyfa stats, item link, contract link (only with
   contract).
4. Image group: "Copy image link", "Download image" (OG PNG).
5. Signed-in only: **DropdownCollectionMenu** — "Collections" submenu
   listing the user's collections (toggle rows: green PlusIcon to add,
   rotated-45 red PlusIcon to remove, trailing external-link to the
   collection) + "Create collection" (creates and adds); collection-note
   add/edit (only for collection owners viewing their collection);
   workbench add/remove; note add/edit; "Set asking price" (owners of
   the public asset); and admin-only "Estimate value" (POST
   `/estimate/{id}`). **[gap]**

## 6. Helpers the page leans on

- **FormatNumber** (`Helper/FormatNumber.ts`) — all number formats are
  pinned to `en-US` deliberately. Port at least: `toIskCompact`
  (Intl compact-long + " ISK", null → "N/A"), `toVeryCompact`
  (compact-short), `toCompact`, `toIsk` (currency style). **[gap]**
- **Export** (`Helper/Export.ts`): Pyfa =
  `"{source_type.name}\n{mutaplasmid.name}\n" + "{name} {value}"`-list
  comma-joined; item link =
  `<url=showinfo:{type_id}//{module_id}>{type_name} ({module_id})</url>`;
  contract link =
  `<url=contract:30000142//{contract_id}>Contract {id} ({type}) {price ISK}</url>`.
  **[gap]**
- **useImage**: OG PNG url (`{origin}/og/module/{id}.png`) + `image_name`
  = `{module.slug}.png` for downloads/copies. **[gap]**
- **AttributeFormatter** — already ported (`frontend/src/lib/attributes.ts`
  mirrors the Rust port; pinned by vitest + Rust tests).
- Toasts: every copy action fires a success notification
  (NotificationStore) — **[gap]** we have no toast system yet.

## 7. Data the API must grow for full parity

Our `ModuleDetail` already reserves `contract`, `public_asset`,
`estimated_value`, `estimated_value_updated_at` (null today) — the card
consumes them as §3.3 describes. Still missing from the payload
entirely: `note`, `collection_note`, `training_module`, `latest_offer`,
`asset.owner`/`asset.station` on the personal rows, and the show-page
props of §1 (estimator statistic for the type, market histories,
historic contracts, deferred similar modules). Grow them feature by
feature with their tests, keys pinned per the testing rules.

## 8. Suggested porting order

1. ✅ `hud-readout`/`hud-scan` utilities + FormatNumber + a toast store
   (everything below uses them).
2. ✅ Card location rows: Contract + EstimatedValue fallback (data already
   served) — replaces the static "Est. value: N/A" footer.
3. ✅ Hero (CreatorDetails + EstimatedValue/MissingData + stat grid) with a
   `GET /api/module-page/{module}` payload serving the type's estimator
   statistic; toolbar with the export/share/copy actions.
4. ✅ Source-types tab, computed server-side from reference tables +
   market histories (`source_type_comparisons`).
5. ✅ Contract-history tab: `historic_contracts` archive filled by the
   region sync's invalidation (ESI status probe), plus the admin
   `PUT /api/historic-contracts/{id}` moderation.
6. ✅ Similar-sold tab: `training_modules` sweep (scheduler job
   `training-modules`), `GET /api/module-page/{module}/similar`
   distance query behind the premium gate, blurred teaser for the rest.
7. ✅ Menus (shared dropdown/context content via `module-menu-items`) +
   search-menu attribute bounds through the shared query builder
   (`module-finder.ts`, fed by `abyssal_type_statistics`).
8. Notes/collection notes/asking price/offers rows + the menu's
   collection/workbench/note/asking-price entries as their backend
   features land (Phase D order).

---

## 9. Leaf components — images and icons (nothing below these)

### 9.1 Image components (`Components/Images/`)

All are `UseImage` (vueuse) wrappers: they render the `<img>` with the
given alt, and swap in **Fallback** on load error. Svelte port: an
`<img>` with an `onerror` state switch to the fallback.

| Component | Props | `src` | Notes |
|---|---|---|---|
| `AbyssalIcon` | `type_id`, `type_name` | **local** `asset("/img/icons/{type_id}.png")` | card header icon; `data-categories="essential"` (consent tooling); alt = type name |
| `AttributeIcon` | `attribute_id`, `attribute_name` | **local** `asset("img/icons/{attribute_id}.png")` | attribute rows, search menus, table headers; alt = attribute name |
| `TypeIcon` | `type_id?`, `type_name?` | `https://images.evetech.net/types/{id}/icon?size=64` | asset/location rows; renders bare Fallback when no `type_id` |
| `CharacterIcon` | `character_id`, `character_name`, `size?=64` | `https://images.evetech.net/characters/{id}/portrait?size={size}` | creator, public-asset owner |
| `Fallback` | `class?` | — | `div.border-border.grid.aspect-square.place-items-center.rounded-lg.border` containing a muted `QuestionIcon` at full size |

`asset(path)` = `VITE_APP_URL` + `/` + path (leading slash stripped) —
ours: the `/img` prefix is served by Axum through the shared origin, so
plain `/img/icons/{id}.png` works. **[gap]** our card currently hotlinks
evetech for the *type* header icon where legacy uses the local abyssal
icon set, and has no error fallback anywhere.

### 9.2 Icon components (`Components/Icons/`)

Every one is the same shape — a lucide glyph at `stroke-width: 1`,
`class="h-[1em] w-[1em]"` (so it scales with font-size and inherits
`currentColor`). Port as one `Icon` wrapper or use `@lucide/svelte`
directly with these mappings:

| Legacy component | Lucide glyph | Seen in |
|---|---|---|
| `AIIcon` | `Cpu` | EstimatedValue/Training rows (green-500), estimate menu item |
| `AuctionIcon` | `Gavel` | Contract row, auction type (amber-500) |
| `ExchangeIcon` | `ArrowLeftRight` | Contract row, item-exchange type (amber-500) |
| `NoteIcon` | `StickyNote` | note rows + menu items (lime-500), NoteMenu bar |
| `CoinIcon` | `Coins` | asking-price row + menu (amber-500) |
| `EllipsisVIcon` | `MoreVertical` | card header menu trigger |
| `QuestionIcon` | `HelpCircle` | image Fallback |
| `CopyIcon` | `Copy` | copy menu items, FindAssetTooltip |
| `CollectionIcon` | `History` | collections submenu trigger |
| `PlusIcon` / `MinusIcon` | `Plus` / `Minus` | collection toggle (green add; red remove is the *same Plus rotated 45°* via `rotate-45 text-red-500`, transition 500 ms), workbench add/remove |
| `SearchIcon` | `Search` | search submenu triggers (`w-4 text-neutral-500` in context menu) |
| `OpenExternalIcon` | `ExternalLink` | share item, collection external links (`h-2.5`) |
| `ImageIcon` | `Image` | image menu items |

The Show toolbar imports lucide directly (Search, GitCompareArrows,
TrendingDown, History, FileCode2, Link, FileSignature, ExternalLink,
Share2, Ellipsis, Image, Download, ChevronDown, MoveRight, Info,
ChartColumn, FileClock, PackageCheck, Check, Cpu) at default stroke.

## 10. Behavior contracts — the composables behind the card

### 10.1 Notes, collection notes, asking prices: global batch-edit mode

These three share one pattern (module-level `useX` + page-level `useXs`
over a module-keyed store):

- "Add/Edit note" in any card menu flips a **global** `is_editing` — the
  editor row opens on *every* card at once; edits accumulate in a store
  keyed by module id (surviving navigation while editing); leaving edit
  mode resets unsaved drafts.
- A **floating save bar** appears while editing (`Components/Notes/
  NoteMenu.vue`, mirrored by `Pricings/PricingMenu.vue` and
  `Collections/CollectionNoteMenu.vue`): fixed `bottom-8 left-1/2
  -translate-x-1/2 z-40`, `bg-card-1 border rounded-md p-4 px-6
  shadow-xl`, colored icon + "Editing enabled" + secondary Cancel +
  primary Save, 0.5 s fade transition.
- Save batches everything: `POST /notes` `{notes: [{module_id,
  content}]}` · `POST /module-pricing` `{module_pricing: [{module_id,
  price}]}` · collection notes analogous via `/collection-notes`
  (collection-scoped). Empty batch = just close.
- Gating: collection notes editable only when the signed-in user's
  characters include the *viewed collection's* creator
  (`page.props.collection.creator.id`); asking price only when a
  character owns `module.public_asset.owner.id` (`can_set_price`).

### 10.2 The rest

- **useWorkbench(module)**: `is_in_workbench` from the shared
  `workbench` prop; add = `POST /workbench-modules` `{module_id}`
  ("Workbench full" error toast on validation failure), remove =
  `DELETE /workbench-modules/{id}`; plus a global `is_open` for the
  workbench drawer. Search-menu submits auto-add the module.
- **useCollections(module)**: joins the page-shared `collections` prop
  (the user's collections) against `module.collections` (which carries
  `collection_module_id` when the module is in one) to render toggle
  rows; add = `POST /collection-modules`, remove =
  `DELETE /collection-modules/{collection_module_id}`, create =
  `POST /collections/modules` with name "New Collection", description
  "New collection", visibility private, `modules: [id]`.
- **useImage(module)**: `image_path` = origin + `/og/module/{id}` +
  `.png`; `image_name` = `{slug}.png`; copy fires success/error toasts.
- **useMakeOffer**: provide/inject singleton (`provideMakeOffer` at the
  layout) holding `{module, is_open}`; `openWithModule(module)` opens
  the make-offer dialog (offers feature, unported).
- **useEstimate(module)**: `estimateValue()` = `POST /estimate/{id}`
  (admin-only menu item; our endpoint already exists).
- **useTeaserModules(module, count=6)**: the premium teaser fakes —
  deep-clones the module `count` times, wiggling each attribute's
  value/fraction by a deterministic pseudo-random factor
  `0.85 + pseudoRandom(index·31 + attrIndex·7) · 0.3`.

## 11. The full ModuleResource shape the components consume

`types/models.types.ts` `TModuleResource` (superset of our
`ModuleDetail`; * = missing from our payload today):

```
id, name*, slug, type, source_type, mutaplasmid, creator,
mutated_attributes[], contract, estimated_value,
estimated_value_updated_at, asset (owner*, station*, parent_*,
location_*), training_module*, collections[]* ({id, name,
collection_module_id, slug}), collection_note* ({id, content,
collection}), public_asset ({owner, price, ...}*), latest_offer*,
note* ({content})
```

`type`/`source_type` also carry `abyssal_type` metadata client-side from
the bundled static data (input type meta groups/levels) — server-side for
us via the reference tables.
