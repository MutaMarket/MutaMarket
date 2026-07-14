# Module Appraisal

## How do I appraise a module?

Visit the [Appraisal Page](/modules/add) and choose one of the following methods to appraise a module:

- **Appraise by Item Link**: Paste the item link of the module you want to appraise and click "Appraise Module."
- **Appraise by Item ID & Type**: Enter the item ID and type of the module you want to appraise and click "Appraise
  Module."
- **Appraise via EveMail**: Send an EVE mail to the character "MutaMate" with the item. The appraisal tool will
  respond with the estimated value.
- **Appraise via ESI**: Import your assets from EVE Online to MutaMarket and appraise modules directly from your
  inventory.

To get a module's item link in-game:

1. Drag the module into a chat window.
2. Send the message.
3. Right-click the message to copy the link.

Go to the [Appraisal Page](/modules/add) and paste the link into the input field. Click "Appraise Module," and the
tool will calculate an estimated value based on historical data. For a more accurate appraisal, compare the module to
similar items on the market by using the "Search for Similar/Cheapest" option. This approach gives you a comprehensive
understanding of the module's value.

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
  Right-click on any module on the site and select "Search for Similar/Cheapest" to find comparable items. Focus on
  stats that are most important to buyers, such as range, damage, or activation cost, depending on the module type.

## How does the appraisal tool work?

The appraisal tool uses a Random Forest Regression (RFR) algorithm to estimate module values based on historical data.
Here's how it works:

### Data Collection

- Historical sales data from MutaMarket
- Module attributes and stats
- Mutaplasmid types used
- Market conditions at time of sale

### The Algorithm

Random Forest Regression:

- Creates multiple decision trees using different subsets of data
- Each tree makes a prediction based on the module's attributes
- The final estimate is the average of all tree predictions
- This helps reduce overfitting and improves accuracy

> **Note:** The model is regularly retrained with new sales data to improve its accuracy over time.

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
