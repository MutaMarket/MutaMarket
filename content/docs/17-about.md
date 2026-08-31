---
section: General
---

# About

I am Nicolas Kion, and I built MutaMarket.

It started because I had the same problem every day: abyssal modules
scattered across characters and containers, no idea what any of them were
worth, and no good way to find the specific roll I wanted. The in-game
tools do not help with any of that.

So MutaMarket is what I wanted to exist. You import your modules from EVE,
it tells you what they are worth based on what similar rolls have actually
sold for, and it gives you somewhere to sell them and somewhere to keep
track of what you have.

## How it is built

The backend is Rust, using Axum and Postgres. The frontend is SvelteKit
with Tailwind. Market and asset data comes from EVE's ESI API.

The price estimates come from a random forest trained per module type on
real recorded sales. [Appraisal](/documentation/appraisal) explains how it
works and where it falls down.

## Getting in touch

The Abyssal Trading Discord is where the trading community is, and I am
there. The MutaMarket development Discord is the place for bugs and feature
requests. Both are linked in the footer.

You can also mail Nicolas Kion in game, or email
[nicolaskion07@gmail.com](mailto:nicolaskion07@gmail.com).
