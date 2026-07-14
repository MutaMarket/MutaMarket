# Selling Modules & Managing Assets

## How do I sell modules on MutaMarket?

There are two independent ways to get a module in front of buyers:

### Option 1: Create an in-game contract

Set up a public contract in-game (item exchange or auction). MutaMarket scans public contracts in all known-space
regions every 30 minutes and automatically lists any abyssal modules it finds on the [market](/modules). No account or
additional steps are required — auction bids and your own character contracts are refreshed even more frequently.

### Option 2: List modules directly on MutaMarket

Direct listings are "public assets": real modules from your inventory that you make visible so buyers can send you
[offers](/documentation/offers). The trade itself is then settled with a normal in-game contract or trade that you and the
buyer agree on.

1. Log in and visit the [Sell Modules page](/sell/modules).
2. Click "Start Import" to import your assets (requires the "Read Assets" ESI scope — the site prompts you to grant it
   if missing).
3. Put the modules you want to sell into containers (cans) or ships in-game. Modules loose in a station hangar won't
   show up.
4. Click "Select modules" to open the "Make your modules visible" dialog. It lists your containers and ships that hold
   abyssal modules; toggle "Is visible" on a container to make it and everything inside it public.
5. Set asking prices: use "Set asking price" on a single module, or "Edit asking prices" on the Sell page to price
   many modules at once. Modules without a price show "Make offer" to buyers instead.

> **Important:**
>
> - For security reasons visibility is toggled per container or ship, never for a whole station. Station containers
>   are the recommended way to organize what's for sale.
> - Only containers that actually contain abyssal modules appear in the selection list. This gives you precise control
>   over which modules and locations are made accessible.
> - Public modules appear on the main market page, on the Sell page, and on your public character page.
> - Check market prices and use the [appraisal tool](/documentation/appraisal) to set competitive prices.

## How do I import and update my assets?

1. Go to [Your Modules](/personal/modules)
2. Click "Start Import" and wait for the import to finish (the page shows the import's progress)

The import fetches your assets through ESI, finds the abyssal modules inside your containers and ships, and adds them
to your MutaMarket inventory. After the first import, the site automatically refreshes each character's assets
periodically (roughly every six hours), so your inventory stays current without frequent manual imports.

If you've granted the corporation scope, modules in corporation hangars are imported as well.

### Troubleshooting asset import

If you are having trouble importing your assets, try logging out of your MutaMarket account and logging back in. This
refreshes your connection to EVE Online and may resolve any issues with the asset import tool. If you continue to
experience problems, feel free to contact us on Discord for assistance.

## What does the Your Modules page show?

[Your Modules](/personal/modules) lists every abyssal module you own, with the full filter system plus location
filters: "Without contracts", "Without fitted", and "Without assets". From each module you can appraise, add it to a
collection or the workbench, attach a private note, set an asking price, or jump to its contract.

## How do I find a module in-game?

Hover over a module's location on the Your Modules page for exact instructions. In short:

1. Open the container named on the module in your in-game inventory.
2. Sort the items by "Type" (the default) and search for the module's type name.
3. Count from the top to the position number shown on MutaMarket — if the number is 4, it's the 4th matching item.

The tooltip also translates the position into a row/column hint if you size your inventory to 10 items per row.

## What are the Locations pages?

The [Locations page](/locations) shows a searchable tree of every station, structure, container, and ship where you
keep abyssal modules, with a module count per location. Click a location to browse all abyssal modules inside it with
the usual filters. From a location page you can click "Create Collection" to snapshot everything in that location into
a new private [collection](/documentation/collections).

## How do I track my contracts?

The [Your Contracts page](/personal/contracts) collects all of your abyssal contracts — outstanding, completed, and
historic — over an adjustable date range. It shows totals for earnings, ISK spent, outstanding contracts and their
value, and your profit, plus a searchable contract table. Use "Refresh contracts" to force a re-scan of your character
contracts.

> Due to the nature of EVE's API, some contracts may be missing data. This is not a bug, but a limitation of the API
> itself.

### How do I open a contract in-game?

1. Right-click a module (or use its detail-page toolbar) and select "Open contract in game", or
2. Select "Copy contract link" and paste it into the in-game notepad — links cannot be pasted into chat.

## What are my personal stats?

The [Statistics page](/personal/stats) summarizes the modules you have rolled across all your characters: total
modules created, estimated ISK spent creating them (base module + mutaplasmid at Jita average prices), and the total
estimated value of what you rolled — plus a searchable breakdown by type and character. Figures are estimates based on
today's Jita averages.
