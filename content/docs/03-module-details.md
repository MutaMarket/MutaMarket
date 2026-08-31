---
section: Modules
---

# The module page

Every module has its own page. Click any listing on the
[market](/modules) to open it. It holds the full stat breakdown, what the
price model thinks it is worth, the tools you use most, and what modules
like it have done on the market.

The page has three parts: the card on the left, the panel beside it, and
the tabs underneath.

## The card

The card is the module itself: its name, the module it was rolled from, the
mutaplasmid used, and every mutated attribute with its rolled value, how far
that moved from the base, and a bar showing where the roll landed in the
possible range.

If it is on contract, the card shows the contract price next to the
estimated value. A `+N` badge means the contract holds other modules too.

Attribute rows are coloured. Green or red says the roll came out better or
worse than the base value. Gold, diamond and brown flag rolls that hit the
type's extremes, which
[Browsing the market](/documentation/browsing-the-market) explains. Turn on
attribute scores in the display settings and each row also gets a signed
score for how far into the range it landed.

## What it is worth

The panel beside the card shows who rolled the module, and what the price
model predicts.

| Field | Meaning |
| --- | --- |
| Estimated value | The predicted price in ISK. Click it to copy. |
| ±% | The model's average error for this module type. |
| Evaluated | When this module's estimate was last worked out. |
| Confidence | Very low to very high, from the model's R² score. |
| Bias score | How evenly the training data covers the source types. |
| Avg. error (MAE) | Mean absolute error. Lower is better. |
| Last trained | When this type's model was last rebuilt. |
| Training data | Recorded trades per source type. Fewer than ten is flagged red. |

A type needs 50 recorded trades before its model can be trained. Below that
there is no prediction, and the panel shows how far off it is instead.

These models can be badly wrong. Treat the number as a starting point and
check what similar modules are actually going for. The
[appraisal guide](/documentation/appraisal) covers where they fall down.

## The toolbar

| Button | What it does |
| --- | --- |
| Search this type | The market, filtered to this module's type. |
| Search similar | Pick the attributes to match and a tolerance, then find modules for sale with rolls like it. |
| Search cheapest | Same, sorted cheapest first. |
| Search historic | Same, against historic sales instead of live listings. Premium. |
| Pyfa | Copies the stats in a format Pyfa understands. |
| Copy item link | Copies a link you can paste into the in-game notepad. |
| Copy contract link | Copies the contract link. Off when there is no contract. |
| Open contract in game | Opens the contract in your EVE client. Off when there is no contract. |
| Share module | Shares or copies a link to this page. |
| More actions | Copy or download the module's share image. |

The three search menus work the same way. Tick the attributes you care
about, set how much variance to allow, and search. "Select all" and
"Clear all" toggle everything at once.

## Card menu

Right-click the card, or use its `⋮` button, for the rest. Logged in, that
adds:

**Collections** puts the module in one of your
[collections](/collections), or a new one, and lets you leave a note on it
there.

**Workbench** adds or removes it from your comparison
[workbench](/documentation/workbench-and-tools).

**Notes** attaches a private note. Only you can see it.

**Set asking price** appears on your own listed assets and sets the price
buyers see.

The copy and export actions from the toolbar are there too.

## Tabs

**Source types** compares every module the mutaplasmid can be applied to,
from T1 up to officer. For each mutated attribute it shows that source's
base value and how this roll compares, plus what each source currently
costs. It is the fastest way to find out whether your roll actually beats
buying the faction version.

**Contract history** is every contract MutaMarket has seen for this exact
module: who issued it, when, whether it held other items, what happened to
it, and the price. The live one sits at the top as outstanding.

**Similar sold** needs [premium](/documentation/premium). It shows modules
with rolls like this one that actually sold, with the average, lowest and
highest price. Without premium you get a blurred preview.

## Buying it

If someone listed the module directly on MutaMarket, the price on the card
is clickable. It shows their asking price, or "Make offer" if they have not
set one. That opens a dialog where you message the seller. If you already
have an offer open on it, the card says "Go to offer" instead.
[Offers](/documentation/offers) has the full flow.

Modules on an in-game contract are bought through the contract. Use "Open
contract in game" or copy the link.
