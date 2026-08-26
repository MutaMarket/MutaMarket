# The browser filter panel — legacy spec for the one-to-one port

Source of truth: `resources/js/Components/Filters/**` in the legacy
project, read component by component. This documents the full tree for
the `modules` (home) and `all-modules` pages; the other page variants
(Calculator, Character, Location, Personal, Sell, Training) reuse the
same pieces and arrive with their features.

Every navigation below goes through the shared query grammar
(`buildQueryPath` in our `frontend/src/lib/query.ts` mirrors legacy
`QueryBuilder.makeFromDefaults`): change one option, keep the rest,
`preserveState/preserveScroll` (ours: `goto` with `keepFocus/noScroll`).
The Rust search grammar already parses every option used below
(including `in-jita` and `with-personal-modules`).

## 1. Page composition — `Pages/ModulesFilters.vue`

A `Card` (relative z-10, `divide-y`), NOT a sidebar — the filters sit as
a band above the module grid:

- Row 1, grid `2xl:grid-cols-[3fr_2fr]`, `divide-x`:
  - Left column (`divide-y`):
    - **GeneralFilter** — `FilterArea` grid `xl:grid-cols-3`:
      TypeDialog (category/type picker), MetaGroupFilter,
      MetaLevelFilter, ImportPyfaModule (calculator import — later).
    - A stacked flip area (`grid *:col-start-1 *:row-start-1` with a
      `Transition`): either the three filter columns
      (**AvailabilityFilters | ContractTypeFilters | MiscFilters**, grid
      `sm:grid-cols-3`) or **ModuleStats** (the stats panel), toggled by
      an icon Button pinned `absolute top-1 right-1` (chart icon ↔
      filter icon, title "Toggle stats").
  - Right column: **PriceFilter** then **ValueFilter**, both with
    hardcoded bounds `lowest=1_000_000`, `highest=100_000_000_000`.
- Row 2 (full width): **AttributeFilters**.

`Pages/AllModulesFilters.vue` differs only in: no flip button/stats
transition (shows `TotalStats` directly), no AvailabilityFilters, no
ContractTypeFilters, no PriceFilter (value + misc + attributes remain).

Frame primitives: `FilterArea` = `div.p-4`; `FilterSection` = plain div;
`FilterTitle` = `h2.mb-2`; `FilterContent` = plain div.

## 2. The custom slider (vue-3-slider-component)

All range filters use one slider engine, 0–100 domain, `interval 0.01`,
with three slots. Our port needs a custom Svelte slider (bits-ui Slider
lacks marks/step slots) with:

- **Track/process**: muted track, primary-colored process segment
  between the two handles; primary dot handles.
- **Marks** dict keyed by 0–100 position, two kinds:
  - `regular`: a text label under the track (`0.625rem`,
    muted-foreground, centered under the position). The first mark label
    is green and the last red (`.reversed` flips green/red — used by the
    price slider, where left = best = cheapest... see §4).
  - `custom` (attribute sliders only): a **pip** — `size-2 rounded-full
    bg-popover ring-2 ring-muted`, centered on the track
    (`top-1/2 -translate-x-1/2 -translate-y-1/2`), `ring-primary` when
    inside the active range — marking where a real source type sits.
    On hover, a popover (`absolute bottom-4 left-1/2 -translate-x-1/2
    bg-popover border rounded-lg p-4 z-50`) shows the formatted
    attribute value (xs uppercase, foreground) plus one line per type at
    that position (`text-xs whitespace-nowrap`), sorted by meta rank
    then name (`sortByMetaAndName`).
- **Tooltip** while dragging a handle: attribute sliders show the
  formatted attribute value (`AttributeTooltip`: `border-primary
  bg-popover rounded-lg border p-2 shadow-lg`); currency sliders show
  `toVeryCompact` ISK (`ValueTooltip`, same look on bg-card).
- Slider changes fire the (debounced 200ms) search; a no-op guard skips
  navigation during SSR.

## 3. Attribute filters — `Attributes/*`

**AttributeFilters** renders below everything, only when a type is
selected (`search.type`), keyed by type id. Grid `xl:grid-cols-2` with
`gap-x-12`; when the attribute count is odd, the empty cell shows a
dashed placeholder border. Rows come from the type's
`abyssal_type_attributes` (per-attribute best/worst), filtered by:
`best !== worst`, and the page's `allow_virtual` / `allow_derived`
flags (both true on browse pages). A floating **SourceTypeAttributeSelect**
button sits dead-center over the grid (§3.5).

### 3.1 AttributeFilter row

`FilterArea` (p-4) containing:
- Title: attribute icon (size-6, `/img/icons/{id}.png`) + display name.
- Right-aligned, max-w-[300px]: **AttributeInput** (two number inputs)
  with a trailing **related-types dropdown** button (§3.4).
- Full-width slider (px-4 pb-4) with marks from §3.2, values from §3.3.
- **SortByAttributeButtons** on the right edge (§6), when sorting is
  allowed.

### 3.2 Marks — `useAttributeMarks`

Positions are the 0–100 normalization of a value between the type's
`worst` (0) and `best` (100) (`mapMinMax`). Two sources:
1. One `custom` pip per distinct normalized position of every **source
   type's base value** for this attribute (from
   `abyssal_type_type_attributes`: rows (type_id, attribute_id, value)
   for the abyssal type's input types). Types stacking on the same
   position share one pip; the hover popover lists them all.
2. `regular` labels at 0/20/40/60/80/100 (skipped when a pip already
   occupies the position), label = the formatted attribute value at that
   position.

### 3.3 Values, search, inputs

- Initial handle positions come from the URL search's attribute bounds,
  normalized and **clamped** to [0,100] (URL bounds can exceed the
  type's range; unclamped values make the slider self-correct and fire a
  phantom navigation). Missing bounds → [0,100]. Lower-bound-only →
  `[normalized(lower), 100]`.
- On change (`useAttributeSearch`, debounced 200ms): untouched
  ([0,100]) drops the attribute from the URL; `[x,100]` emits
  lower-bound-only; otherwise both bounds, min/maxed. Bounds are
  denormalized back through best/worst.
- **AttributeInput**: two bordered number inputs (h-8, right-aligned
  text-xs, unit `display_name` overlaid left as muted text, spinner
  arrows hidden), showing the *display-transformed* true values
  (`transformValue`, e.g. ms→s). Enter or blur submits: values are
  re-verted (`revertTransformValue`) and normalized. Changing the
  selected type resets both inputs.

### 3.4 Related-types dropdown (per attribute)

Trailing icon button (list icon, `rounded-l-none`, attached to the
inputs). Menu: every source type carrying this attribute — meta-group
dot (gray/orange/green-300/green-500/purple/blue for 1/2/3/4/5/6) +
name, meta-rank-then-name order. Clicking one sets the slider to
`[normalized(type value), 100]` ("at least as good as X") and searches.

### 3.5 SourceTypeAttributeSelect (the center button)

`absolute top-1/2 left-1/2` over the attribute grid, xl+ only: list-icon
button opening a w-72 p-4 dropdown: a Select of the source types
(meta dot + name), a "Select all" switch plus one switch per visible
attribute (icon + display name), and an "Apply" button (disabled when
none selected). Submit: for each checked attribute, a lower-bound-only
filter at the chosen type's base value — "at least as good as this
type" across many attributes at once.

## 4. Price and estimated value — `PriceFilter` / `ValueFilter`

Same skeleton as an attribute row: wallet-icon title ("Price" /
"Estimated value"), **CurrencyInput** (max-w-[300px]) and a slider, plus
SortByPriceButtons / SortByValueButtons.

- **Log scale**: position ↔ ISK via `CurrencyMapper`
  (`100 · (log10(v+1) − log10(lowest+1)) / (log10(highest+1) − log10(lowest+1))`
  and its inverse). Marks every 20 positions, label `toVeryCompact` of
  the denormalized ISK.
- **Price slider is `reversed`** (the wrapper class flips the first/last
  label colors green↔red): dragging only the *right* handle
  (`[0, x]`) means "at most X" and emits a lower-bound-only price — the
  legacy price URL segment's single bound is a *maximum*. Both handles
  → both bounds.
- **Value slider** is normal: `[x, 100]` → lower-bound-only ("worth at
  least X"); both handles → both bounds. Untouched drops the segment.
- **CurrencyInput**: two compact-formatted number inputs; typing `m`
  multiplies the focused input by 1e6, `b` by 1e9; Enter/blur submits;
  type change resets. (Displays `Intl compact` of the current bound.)
- Changing the selected type resets both currency sliders.

## 5. Select filters

- **MetaGroupFilter**: Select "All" + meta groups (name + colored dot),
  narrowed to the groups present among the type's input types
  (`input_type_meta_groups`) when a type is selected. Emits
  `meta-group/{id}` (null for All).
- **MetaLevelFilter**: Select "All" + named meta levels, each showing
  the dots of the groups it contains, narrowed like meta groups via
  `input_type_meta_levels`. Emits `meta-level/{n}`.
- **ContractTypeFilters**: radio group All / Item exchange / Auction.
- **AvailabilityFilters**: switches — Personal modules (disabled for
  guests), Only contracts, Multi-item contracts (inverted:
  off ⇒ `no-multi-item-contracts`), "Jita 4-4" (`in-jita`).
- **MiscFilters**: switches — Gold bar, Brown bar, Diamond bar rolls.

## 6. Sort buttons — `SortByButtons`

A vertical trio on the right edge of a slider row: chevron-up button,
tiny uppercase "SORT" label, chevron-down button. The active direction
gets `text-primary animate-pulse`; clicking the active direction links
to the *unsorted* URL (toggle off), otherwise to sort asc/desc.
Variants: price (`sort/price/{dir}`), value (`sort/value/{dir}`),
attribute (`sort/{attribute-name}/{dir}`; active only when the sort is
that attribute).

## 7. Data the frontend needs (backend gaps)

Our `/api/filter-panel/{type}` already serves per-attribute best/worst
(the `abyssal_type_statistics` port). Missing for one-to-one:

1. **Source-type base values per attribute** (legacy client-bundled
   `abyssal_type_type_attributes`): rows `(type_id, attribute_id,
   value)` for the abyssal type's published input types, plus the types'
   `(id, name, meta_group_id)` — powers pips (§3.2), the related-types
   dropdown (§3.4) and the center select (§3.5). Derivable from
   `mutaplasmid_input_types` × `type_attributes`; extend the
   filter-panel payload.
2. **`input_type_meta_groups` / `input_type_meta_levels`** of the
   selected type — narrows the two selects (§5). Same join, aggregated
   (meta level = dogma attribute 633).
3. Nothing else: sort/price/value/attribute grammar, bar switches,
   in-jita and personal-modules are already parsed by the Rust search.

## 8. Porting order

1. The custom slider component (track/process/handles, regular marks,
   custom pips + hover popover, drag tooltips, reversed mode) + unit
   tests for the mappers (`CurrencyMapper` log scale, clamping).
2. Filter-panel API extension (§7.1, §7.2) with exact-key tests.
3. Attribute rows: slider + inputs + related-types dropdown + sort
   buttons, replacing the current plain-slider sidebar entries.
4. Price/value rows with the log scale, reversed price semantics, and
   currency inputs (m/b shortcuts).
5. Page recomposition: the filter band layout of §1 (card above the
   grid, stats flip), meta selects narrowed, availability/contract/misc
   groups.
6. SourceTypeAttributeSelect.
7. Later, with their features: ImportPyfaModule (calculator), the other
   page variants, personal-modules switch enablement.
