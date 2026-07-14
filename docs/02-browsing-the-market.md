# Browsing the Market

## What does the modules page show?

The [Modules page](/modules) is the marketplace view. It lists every abyssal module that is currently available:
modules on public in-game contracts (item exchanges and auctions) and modules listed directly on MutaMarket as public
assets by their owners. Results are paginated, 40 modules per page.

Each module card shows the source type and mutaplasmid used (with a colored border indicating the meta group), every
mutated attribute with its value and the difference against the base module, a roll bar per attribute, and the price:
contract listings show the contract price with an item-exchange or auction icon, plus the estimated value; multi-item
contracts carry a `+N` badge for the number of additional modules in the contract.

Click any card to open its [detail page](/documentation/module-details). Right-click a card (or use its `⋮` button) for quick
actions: search for similar/cheapest/historic, share, open or copy the contract, copy Pyfa stats or an item link, and —
when logged in — collections, workbench, notes, and asking price.

## How do filters work?

All filters live in the URL, so any filtered view can be shared, bookmarked, or re-opened later. For example,
`/modules/type/500mn-abyssal-microwarpdrive/sort/price/asc` shows all 500MN abyssal MWDs sorted by cheapest first.

| Filter | UI | URL segment |
| --- | --- | --- |
| Type | "Category" picker | `type/<type-name>` |
| Meta group | "Meta Group" (All, T1, T2, storyline, faction, officer, deadspace) | `meta-group/<group>` |
| Meta level | "Meta Level" | `meta-level/<n>` |
| Attributes | Per-attribute range sliders (shown once a type is selected) | `attributes/<name>/<min>-<max>/...` |
| Sort | Sort by price, estimated value, or any attribute, ascending/descending | `sort/<field>/<asc\|desc>` |
| Price | "Price" range slider (1 million – 100 billion ISK) | `contract-price/<min>-<max>` |
| Estimated value | "Estimated value" range slider | `estimated-value/<min>-<max>` |
| Contract type | "Contract type": All / Item exchange / Auction | `item-exchange` or `auction` |
| Only contracts | "Only Contracts" switch (hides direct listings) | `contracts-only` |
| Multi-item contracts | "Multi Item Contracts" switch (off = single-item contracts only) | `no-multi-item-contracts` |
| Personal modules | "Personal Modules" switch (include your own imported assets; login required) | `with-personal-modules` |
| Jita | "Jita 4-4" switch (only modules located in Jita 4-4) | `in-jita` |
| Goldbar / Brownbar / Diamondbar | "Miscellaneous" switches | `goldbar`, `brownbar`, `diamondbar` |

You can also import a module from Pyfa ("Import Pyfa module" in the filter panel) to search for modules with similar
stats.

### What are goldbars, brownbars, and diamondbars?

These flag extreme rolls on individual attributes:

- **Goldbar** — the attribute rolled the best possible value achievable for that source type and mutaplasmid.
- **Diamondbar** — the best possible roll achieved with a Glorified mutaplasmid.
- **Brownbar** — the attribute rolled the worst possible value.

Weak mutaplasmid grades (Decayed, Gravid, and their Glorified variants, Radical, Exigent) never produce gold or
diamond bars. On module cards these rolls are highlighted in gold, diamond blue, or brown on both the value difference
and the roll bar. The "Miscellaneous" filters restrict results to modules that have at least one attribute with the
selected bar.

## How do I change how modules are displayed?

The display options above the module list are remembered across visits:

- **Display**: Grid, List, or Table view.
- **Attribute Bars**: Default (roll position within the mutaplasmid range), Type (normalized across the type),
  Absolute, or None.
- **Show Attribute Scores**: toggles a per-attribute roll-quality score on each module card.

## What do the market stats show?

Toggling the stats panel in the filter area shows live totals: modules in the database, goldbar/brownbar/diamondbar
counts, active contracts, item exchanges, auctions, and how many modules were added in the last hour, day, and week.

## What is the All Modules page?

The [All Modules page](/all-modules) lists every abyssal module in the MutaMarket database — not just the ones
currently for sale. Because there are no contracts involved, its filter set is reduced: type/meta filters, the
goldbar/brownbar/diamondbar switches, estimated value, and attribute ranges. Use it to research what rolls exist,
regardless of availability.

## What is the Characters page?

The [Characters page](/characters) is a directory of everyone currently selling modules directly on MutaMarket
(premium sellers are listed first). Click a character to open their profile: a browsable, filterable view of their
public modules, their description, and — if they've made them public — their linked Discord, Twitch, and Patreon
accounts. With the "created" filter, a character page instead shows the modules that character rolled.

Your own character page additionally lets you edit your description and manage which of your asset locations are
public.

## What is the Statistics page?

The [Statistics page](/statistics) is a "Top Characters" leaderboard ranking characters by the number of abyssal
modules they created (rolled). You can restrict the ranking to a single module type with the Category picker and search
characters by name. Clicking a character's module count opens their character page filtered to the modules they
created.
