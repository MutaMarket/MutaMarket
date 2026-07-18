---
section: Modules
---

# Module Appraisal

## How do I appraise a module?

Visit the [Appraisal page](/modules/add) and use any of these methods:

- **Paste an item link**: Copy a module link from EVE chat and paste it into the input field, then click "Appraise".
  The expected format is an in-game item link (`showinfo:typeID//itemID`).
- **Quick paste**: Press Ctrl+V (Cmd+V on Mac) anywhere on MutaMarket to appraise a module link straight from your
  clipboard — you don't even need to be on the appraisal page.
- **Send via EVE Mail**: Mail your module links in-game to the character **MutaMate**. You'll receive a reply mail
  ("Modules processed") with a MutaMarket link and the estimated value for every module in your mail. You can include
  multiple module links in one mail.
- **Import from assets**: Log in with EVE SSO and import your in-game assets via
  [Your Modules](/personal/modules) to appraise modules directly from your inventory.

To get a module's item link in-game:

1. Drag the module into a chat window.
2. Send the message.
3. Right-click the message to copy the link.

You do not need an account to appraise by item link or quick paste. After appraising, the module gets its own public
[detail page](/documentation/module-details) on MutaMarket with the full AI value prediction, and you're redirected there.

For a more accurate appraisal, use "Search for similar" or "Search for cheapest" on the module page to compare against
what's actually on the market right now.

## What factors affect a module's value?

The value of an abyssal module depends on several factors beyond just its stats. Here's what to consider:

- **Base Module**: The base module contributes inherited stats that can significantly affect value. For example,
  X-Type MWDs are particularly valuable because they lack the capacitor penalty present in other MWDs, making them
  more desirable.
- **Supply of Mutaplasmids**: If specific mutaplasmids are in short supply, even a mediocre roll may eventually sell.
  The availability of mutaplasmids can influence demand and pricing for rolled modules.
- **Current market conditions**: Market saturation plays a big role in pricing. If there are many similar modules on
  the market, prices will drop, especially for modules with lower demand. Keep an eye on market trends to set
  realistic expectations.

### Tools for appraising modules

- **MutaMarket Appraisal Tool**: Use the module appraisal tool on the site to get an estimate of your module's value.
  This tool leverages historical data to provide a rough valuation. While not exact, it gives a reliable starting
  point.
- **Market Comparisons**: For the most accurate appraisal, compare your module to similar listings on the market.
  Right-click on any module on the site and select "Search for similar" or "Search for cheapest" to find comparable
  items. Focus on stats that are most important to buyers, such as range, damage, or activation cost, depending on the
  module type.
- **Historic sales**: [Premium](/documentation/premium) members can compare against modules that actually sold, via
  [Historic sales](/historic-sales) and the "Similar sold" tab on every module page.

## How does the appraisal tool work?

The estimate is produced by a machine-learning model — a Random Forest Regression — trained separately for every
abyssal module type on recorded trades:

- The training data comes from real single-module contract sales that MutaMarket has tracked, plus base-module market
  prices as reference points.
- The model's features are the module's mutated attribute values; the prediction target is the sale price.
- Random Forest Regression builds many decision trees on different subsets of the data and averages their
  predictions, which reduces overfitting.
- A model is only trained once a type has at least 50 recorded trades. Below that, modules of that type show
  "No AI prediction available" instead of an estimate.
- Models are retrained regularly as new sales data comes in.

Every prediction is published together with its quality metadata on the module page: confidence (R²), average error
(MAE), a bias score describing how evenly the training data covers the type's source variants, the number of training
samples, and when the model was last trained. See [the module detail page](/documentation/module-details) for what each field
means.

## What are the limitations of the appraisal tool?

Since the appraisal tool is trained on historical data, it may not always reflect the current market conditions.
Especially for high-value modules, it's recommended to compare your module to similar listings on the market to get a
more accurate appraisal. Important limitations to be aware of:

- **Historical Data**: The tool relies on past sales data, which may not reflect current market conditions
- **Rare Modules**: Very rare or unique modules may have limited historical data, leading to less accurate estimates
- **Market Volatility**: Sudden market changes or events can make historical data less relevant
- **Stat Combinations**: Some unique stat combinations may have too few examples to provide accurate estimates

> **Important:** The appraisal tool provides estimates, not definitive prices. It is best used as a starting point for
> pricing your module, rather than a definitive value. Always check current market prices and similar listings for the
> most accurate valuation.

## Why might the estimated value be wrong?

Several factors can cause the estimated value to differ from actual market prices:

### Market Factors

- Recent market changes not yet reflected in historical data
- Supply and demand imbalances
- Changes in meta or game mechanics affecting module value

### Data Limitations

- Limited historical data for certain module types
- Outliers or unusual sales affecting the average
- Missing or incomplete sales data

### Module-Specific Factors

- Unique stat combinations with few comparable sales
- Special cases like perfect rolls or extremely rare modules
- Changes in module meta or usage patterns

> **Best Practice:** Always use the appraisal tool as a starting point and verify prices against current market
> listings and similar modules.
