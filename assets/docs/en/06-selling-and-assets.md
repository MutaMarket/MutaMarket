---
section: Trading
---

# Selling and your assets

There are two ways to put a module in front of buyers, and they work
independently.

## In-game contracts

Make a public contract in game, item exchange or auction. MutaMarket scans
public contracts across known space every 30 minutes and lists any abyssal
modules it finds. You do not need an account and there is nothing to set
up. Auction bids and your own characters' contracts refresh more often
than that.

## Listing on MutaMarket

The other way is to make modules in your inventory visible so buyers can
send you [offers](/documentation/offers). The trade itself still happens in
game, through a contract or a direct trade you agree between you.

Log in and go to [Sell modules](/sell/modules). Click "Start Import" to
pull in your assets. This needs the read-assets permission, and the site
will ask for it if you have not granted it.

Then put the modules you want to sell into a container or a ship. Modules
sitting loose in a station hangar will not show up. Click "Select modules",
find the container, and turn its visibility on. Everything inside it
becomes public.

To put prices on them, use "Edit asking prices" at the top of the sell
page, or "Set asking price" in a module's menu. Either turns on a price
field per module; the bar at the bottom saves them all at once. A module
without a price shows buyers "Make offer" instead, and they name a number
themselves.

Visibility is deliberately per container or per ship, never for a whole
station, and only containers that actually hold abyssal modules show up in
the list. That is what keeps you from exposing a hangar by accident.

Once public, your modules appear on the market, on the sell page, and on
your character page.

## Importing your assets

[Your modules](/personal/modules) has the same "Start Import" button, and
shows progress while it runs. It reads your assets through EVE's API, finds
the abyssal modules inside your containers and ships, and adds them to your
inventory here.

After the first import it refreshes on its own, roughly every six hours,
so you should not need to do it by hand again. If you granted corporation
access, corporation hangars come along too.

If an import will not work, log out of MutaMarket and back in. That renews
the connection to EVE and fixes most of it. If it keeps failing, come and
say so on Discord.

## Your modules

[Your modules](/personal/modules) lists everything abyssal you own. It has
the usual filters plus three for location: without contracts, without
fitted, and without assets. From any module you can appraise it, put it in
a collection or the workbench, or jump to its contract.

## Finding a module in game

Hover a module's location for exact directions. The short version: open the
container it names, sort by type, which is the default, search for the type
name, and count down to the position number MutaMarket shows. Position 4
means the fourth one down.

The tooltip also gives you a row and column if you size your inventory to
ten items per row.

## Locations

[Locations](/locations) is a searchable tree of every station, structure,
container and ship where you keep abyssal modules, with a count for each.
Open one to browse what is inside it with the usual filters.

There is a "Create collection" button on each location, which snapshots
everything in it into a new private
[collection](/documentation/collections).

## Your contracts

[Your contracts](/personal/contracts) gathers your abyssal contracts over a
date range you pick: outstanding, completed and historic. It totals what
you earned, what you spent, what is still outstanding and what that is
worth, and your profit, over a searchable table. "Refresh contracts" forces
a re-scan.

Some contracts come back missing data. That is EVE's API rather than
anything on our end, and there is no fix from here.

To open one in game, right-click the module and pick "Open contract in
game". Or copy the contract link and paste it into the in-game notepad.
Links will not paste into chat.

## Your stats

[Your stats](/personal/stats) sums up what you have rolled across all your
characters: how many modules, what they cost you to make, and what they are
worth now, with a breakdown by type and character.

The cost is the base module plus the mutaplasmid at today's Jita average,
not what you actually paid, so treat both numbers as estimates.
