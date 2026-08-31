---
section: Modules
---

# Appraising a module

There are four ways to get a module onto MutaMarket and find out what it is
worth. None of the first three need an account.

**Paste an item link.** Go to the [appraisal page](/modules/add), paste the
link, hit Appraise. On that page you can also just press Ctrl+V (Cmd+V on a
Mac) without clicking into the box first.

To get the link in game, drag the module into a chat window, send the
message, then right-click it and copy.

**Mail it in.** Send your module links in game to **MutaMate**. You get a
reply mail with a MutaMarket link and an estimate for each one. Several
links in one mail is fine.

**Import your assets.** Log in and pull your modules straight out of your
in-game inventory from [Your modules](/personal/modules).

Whichever way you use, the module ends up with its own public
[page](/documentation/module-details), and you land on it.

## What actually drives the price

The stats are only part of it.

The **module you rolled from** carries its own stats through. X-Type MWDs
are worth more than you might expect because they have no capacitor
penalty, and no roll changes that.

**Mutaplasmid supply** matters. When a mutaplasmid is scarce, even a
mediocre roll finds a buyer eventually.

**How many are already for sale** matters most of the time. A crowded
market pushes prices down, and it pushes hardest on modules nobody
particularly wants.

So the estimate is a starting point. To price something properly, use
"Search similar" or "Search cheapest" on the module page and look at what
is actually listed. With [premium](/documentation/premium) you can go
further and compare against what genuinely sold, through
[historic sales](/historic-sales) and the "Similar sold" tab.

## How the estimate is made

Each abyssal type gets its own model, a random forest trained on real
sales.

The training data is single-module contract sales MutaMarket has tracked,
with base module prices as reference points. The inputs are the module's
mutated attribute values and the target is the sale price. A random forest
builds a lot of decision trees on different slices of that data and
averages them, which stops any one tree overfitting.

A type needs 50 recorded trades before it gets a model. Under that, its
modules show how far off they are instead of a price. Models are retrained
as new sales come in.

Every prediction ships with its own quality numbers on the module page, so
you can see how much to trust it. [The module
page](/documentation/module-details) explains each one.

## Where it goes wrong

The model only knows what has already sold, so it is behind the market by
however long it takes for sales to accumulate. That matters most when
something changes fast: a patch that shifts the meta, a sudden glut, a
mutaplasmid that got rare.

It is also weakest exactly where you most want it. Rare modules have few
comparable sales. Unusual stat combinations may have almost none. A perfect
roll is, by definition, something the model has barely seen. High-value
modules are where the error in ISK is largest and where you should be doing
your own research anyway.

A single odd sale can drag a type's numbers around, and MutaMarket only
sees the contracts it can see, so the data has holes.

Use it to know roughly which bracket you are in, then check the market.
