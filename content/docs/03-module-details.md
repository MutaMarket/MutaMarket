---
section: Modules
---

# The Module Detail Page

Every module on MutaMarket has its own detail page. Click any listing on the [market](/modules) to open it. The page
combines the module's full stat breakdown, an AI value prediction, quick-action tools, and market history for the
module's type.

## What does the page show?

The page is split into three areas:

- **The module card** — the module's name, base type, the mutaplasmid used (color-coded by meta group), every mutated
  attribute with its rolled value, the difference against the base module, and a roll bar showing where the roll landed
  within the mutaplasmid's possible range. If the module is on contract, the card shows the contract price next to the
  estimated value; multi-item contracts are marked with a `+N` badge.
- **The hero panel** — who created the page ("Created by" with a character portrait; premium members' names show in
  gold), the AI value prediction, and the icon toolbar.
- **The tabs** — Source types, Contract history, and Similar sold (see below).

### Roll quality indicators

Each attribute row shows a colored difference and a roll bar:

- Green/red indicate a better or worse roll than the base value.
- Gold and diamond highlights flag exceptional rolls at the top of the possible range; brown flags rolls at the bottom.
- If you enable "attribute scores" in your display settings, each attribute also gets a signed score derived from how
  far into the roll range it landed, colored green, yellow, or red.

You can switch between three roll-bar modes (default, type-normalized, and absolute) in your display settings.

## The AI value prediction

The "AI value prediction" panel shows what MutaMarket's machine-learning model thinks the module is worth, along with
enough metadata to judge how much to trust it:

| Field | Meaning |
| --- | --- |
| Estimated value | The predicted price in ISK. Click it to copy it to your clipboard. |
| ±% | The model's normalized average error for this module type. |
| Evaluated ... ago | When this module's estimate was last computed. |
| Confidence | A rating from Very low to Very high, derived from the model's R² score. |
| Bias score | How evenly the training data is distributed across source types. |
| Avg. error (MAE) | The model's mean absolute error — the lower, the better. |
| Last trained | When the model for this type was last retrained. |
| Training data | Recorded trades per source type. Types with fewer than 10 recorded trades are flagged in red. |

The panel links directly to the historic sales search for the module's type ("View historic sales").

If a module type has fewer than 50 recorded trades, no prediction is shown. Instead you'll see a "Missing data" notice
with the current trade count.

As the tooltip on the panel says: these models can be very inaccurate, so always do your own research by looking for
similar modules on contracts. See the [appraisal guide](/documentation/appraisal) for the model's limitations.

## The icon toolbar

The toolbar in the hero panel gives one-click access to the most common actions:

| Button | What it does |
| --- | --- |
| Search type | Opens the market filtered to this module's type. |
| Search for similar | Opens a menu where you pick which attributes to match and a variance percentage, then searches modules for sale with similar rolls. |
| Search for cheapest | Same attribute/variance menu, sorted to find the cheapest matching modules. |
| Search for historic | Same menu, but searches historic sales instead of active listings (premium). |
| Pyfa | Copies the module's stats in a format you can paste into Pyfa. |
| Item Link | Copies an item link you can paste into the in-game notepad. |
| Contract Link | Copies the contract link (disabled when the module has no active contract). |
| Open contract | Opens the contract directly in your EVE client (disabled when there is no active contract). |
| Share module | Shares or copies a link to this page. |
| More | Copy or download the module's share image. |

The search menus all work the same way: choose the attributes to match, set a "Variance (%)" tolerance, and search.
"Select all" and "Clear all" toggle every attribute at once.

### Card menu actions

Right-click the module card (or use the `⋮` button on it) for more actions when logged in:

- **Collections** — add the module to any of your [collections](/collections) or create a new one, and add or edit a
  collection note.
- **Add to workbench / Remove from workbench** — put the module on your comparison workbench.
- **Add note / Edit note** — attach a private note to the module. Notes are only visible to you.
- **Set asking price** — only shown on your own publicly listed assets; sets the price buyers see.
- Plus the same copy/export actions as the toolbar (Pyfa stats, item link, contract link, open contract in game, image
  link/download).

## The tabs

### Source types

A comparison table across all source types the mutaplasmid can be applied to (T1, T2, storyline, faction, deadspace,
officer). For every mutated attribute it shows each source type's base value and the difference against this module,
color-coded, plus each type's current market price. This is the quickest way to see whether a roll actually beats the
faction or officer alternative.

### Contract history

Every contract MutaMarket has seen for this exact module: contract ID, issuer, issue and expiry dates, whether it was a
multi-item contract, its status (outstanding, completed, ...), and the price. The current active contract appears at
the top as "outstanding".

### Similar sold (premium)

For [premium](/documentation/premium) members, this tab shows modules with rolls like this one that actually sold, together with
average, lowest, and highest sale price at a glance and a "View historic sales" button. Without premium, the tab shows
a blurred preview and an "Upgrade to Premium" button.

## Making an offer

When a module is listed as a public asset by another user, its price on the card is clickable — it shows the seller's
asking price, or "Make offer" if they haven't set one. Clicking opens the "Make an offer" dialog, where you send a
message to the seller to negotiate. If you already have an open offer on the module, the card links to it with
"Go to offer" instead. See [Offers](/documentation/offers) for the full flow.

Modules listed via in-game contract are bought through the contract itself — use "Open contract" or "Contract Link"
from the toolbar.
