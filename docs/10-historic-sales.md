# Historic Sales

## What is the Historic Sales page?

The [Historic Sales page](/historic-sales) lets you browse every recorded abyssal module sale MutaMarket has tracked —
real modules with the price they actually sold for and when. This is the ground truth behind the AI value predictions,
and the best reference for pricing a module realistically.

Historic sales are a [Premium](/documentation/premium) feature. Without premium, visiting the page redirects you to the
premium page.

## Where does the data come from?

MutaMarket continuously scans public contracts across known space. When a single-module contract disappears, the site
checks against EVE's API whether it was completed (sold) or just expired or was deleted. Completed sales are recorded
with their final price and become part of the historic dataset — the same data the appraisal models are trained on.

## How do I search historic sales?

The page uses the same URL-based filter system as the market: filter by module type and attribute ranges, filter by
sale price, and sort by price or by sale date (newest first by default).

The fastest way in is usually from a module you're evaluating:

- Use "Search for historic" on any module (toolbar or right-click menu), pick the attributes to match and a variance,
  and jump straight to comparable sold modules.
- The "Similar sold" tab on every [module detail page](/documentation/module-details) shows sold modules with rolls like the
  one you're looking at, with the average, lowest, and highest sale price at a glance.
- The "Training data" panel on module pages links to the historic sales for that module's type.

## Why do estimated values and historic prices differ?

Historic prices are individual sales — they include lucky sales, fire sales, and everything in between. The AI
estimate is a model average over many sales. When they disagree, look at several recent comparable sales and judge
where your module's roll sits between them.
