---
section: Modules
---

# Browsing the market

The [modules page](/modules) shows what is for sale right now. That means
public in-game contracts, both item exchanges and auctions, plus modules
people have listed directly on MutaMarket. Thirty per page.

Each card shows the source module and the mutaplasmid used, every mutated
attribute with its value and how far it moved from the base, a bar per
attribute showing how the roll landed, and the price. Contract listings show
the contract price next to an item exchange or auction icon, along with the
estimated value. If a contract holds more than one module, the card carries
a `+N` badge for the rest.

Click a card to open its [detail page](/documentation/module-details).
Right-click it, or use the `⋮` button, for the quick actions: search for
similar, cheapest or historic sales, share it, open or copy the contract,
copy Pyfa stats or an item link. Logged in you also get collections, the
workbench and collections.

## Filters

Filters live in the URL, so any view you build can be bookmarked, shared or
reopened later. `/modules/type/500mn-abyssal-microwarpdrive/sort/price/asc`
is every 500MN abyssal MWD, cheapest first.

| Filter | In the UI | In the URL |
| --- | --- | --- |
| Type | Category picker | `type/<type-name>` |
| Meta group | Meta Group: All, T1, T2, storyline, faction, officer, deadspace | `meta-group/<group>` |
| Meta level | Meta Level | `meta-level/<n>` |
| Attributes | Range sliders, once a type is picked | `attributes/<name>/<min>-<max>/...` |
| Sort | Price, estimated value, or any attribute, either direction | `sort/<field>/<asc\|desc>` |
| Price | Price slider, 1 million to 100 billion ISK | `contract-price/<min>-<max>` |
| Estimated value | Estimated value slider | `estimated-value/<min>-<max>` |
| Contract type | All, item exchange, or auction | `item-exchange` or `auction` |
| Only contracts | Hides direct listings | `contracts-only` |
| Multi-item contracts | Off means single-item contracts only | `no-multi-item-contracts` |
| Personal modules | Include your own imported assets. Needs an account | `with-personal-modules` |
| Jita | Only modules sitting in Jita 4-4 | `in-jita` |
| Gold, brown, diamond bars | Under Miscellaneous | `goldbar`, `brownbar`, `diamondbar` |

There is also an "Import Pyfa module" button in the filter panel, which
takes a module out of Pyfa and searches for ones with similar stats.

## Gold, brown and diamond bars

These mark rolls that hit an extreme of what the abyssal type can reach,
across every mutaplasmid that produces it.

Only the strongest grade for a module earns one. Decayed and Gravid lose
to Unstable on the same module, so they never get a bar, and neither do
Glorified Decayed or Glorified Gravid. Unstable does, and so do Exigent and
Radical, which have no stronger grade above them.

A **gold bar** is a best-possible roll. A **diamond bar** is the same roll
on a Glorified mutaplasmid. A **brown bar** is a worst-possible roll.

An attribute that cannot vary gets no bar at all.

On a card these are coloured gold, diamond blue or brown, on both the value
and the bar. The Miscellaneous filters narrow the list to modules carrying
the bar you picked on at least one attribute.

## Changing the display

The options above the list stick between visits.

**Display** switches between grid, list and table.

**Attribute bars** changes what the bar measures: Default draws where the
roll landed inside your mutaplasmid's range, Type normalises across the
whole abyssal type, Absolute uses the raw value, and None hides the bars.

**Show attribute scores** puts a roll-quality score on each attribute.

## Market stats

The stats panel in the filter area shows live totals: how many modules are
in the database, how many carry each bar, how many contracts are active,
split by item exchange and auction, and how many modules turned up in the
last hour, day and week.

## All modules

[All modules](/all-modules) covers everything MutaMarket knows about, not
just what is for sale. No contracts are involved, so the filters are
narrower: type and meta filters, the bar switches, estimated value and
attribute ranges. It is the page for asking what rolls exist at all.

## Characters

[Characters](/characters) lists everyone selling directly on MutaMarket,
premium sellers first. Open one to see their public modules, filterable like
any other list, their description, and any Discord, Twitch or Patreon
accounts they have chosen to show.

Switch to the "created" filter and the page shows what that character
rolled instead of what they are selling.

On your own page you can edit your description and choose which of your
asset locations are public.

## Statistics

[Statistics](/statistics) ranks characters by how many abyssal modules they
have rolled. You can narrow the ranking to one module type, or search for a
character by name. Clicking someone's module count opens their page filtered
to what they created.
