---
section: Trading
---

# Historic sales

[Historic sales](/historic-sales) is every abyssal module sale MutaMarket
has recorded: the module, what it actually sold for, and when. It is the
data the price models are trained on, and the best thing to price against.

It needs [premium](/documentation/premium). Without it the page sends you
to the premium page.

## Where the data comes from

MutaMarket scans public contracts across known space. When a contract
holding a single module disappears, the site asks EVE's API what happened
to it. If it completed, the sale is recorded at its final price. If it
expired or was deleted, nothing is recorded.

That is why the dataset is single-module contracts only. A contract with
four modules and a ship in it has no price that belongs to any one module.

## Searching it

Same URL-based filters as the market: type, attribute ranges, sale price,
sorted by price or by date, newest first.

Usually you get here from a module rather than starting cold. "Search
historic" in any module's toolbar or right-click menu lets you pick which
attributes to match and how much variance to allow, and drops you on the
comparable sales. The "Similar sold" tab on a
[module page](/documentation/module-details) does the same thing with the
average, lowest and highest price already worked out. The training data
panel links to the sales for that type.

## Why this disagrees with the estimate

Historic prices are individual sales, so they include the lucky ones and
the fire sales. The estimate is a model averaging over many of them.

When the two disagree, the sales are the better evidence. Look at several
recent comparable ones and work out where your roll sits between them.
