---
section: API
---

# API: Filtering and sorting

`GET /api/modules/{query}` takes its filters and sort as URL segments
rather than query parameters. This page is the vocabulary; see the
[endpoint reference](/documentation/api) for the request and response
shapes.

## Options

The `query` path segment **must** contain a type option
(`type/{id-or-slug}`, e.g. `type/49738` or
`type/abyssal-ballistic-control-system`) and accepts the same filter
options as the module browser, chained as URL segments in any order.

| Option | Format | Effect |
|---|---|---|
| sort | `sort/{field}/{direction}` | Sort by `price` (contract price), `value` (estimated value), `fraction` (average roll quality), `contract-date` (when the current contract was issued), `date-added` (when the module's current contract was added to MutaMarket), or a dogma attribute by id or name (`sort/50/desc`, `sort/cpu/asc`). Direction is `asc` or `desc`. Sorting by an attribute only returns modules that have it. |
| attributes | `attributes/{attribute}/{value}` (pairs, repeatable) | Filter by rolled values, e.g. `attributes/cpu/20-30/damageMultiplier/2.1`. A `min-max` range bounds the value; a single number is a minimum where high is good, otherwise a maximum. |
| meta-group | `meta-group/{group}` | One of `t1`, `t2`, `storyline`, `faction`, `officer`, `deadspace`: only modules mutated from a source module of that meta group. |
| meta-level | `meta-level/{n}` | Only modules mutated from a source module of that meta level. |
| contract-price | `contract-price/{max}` or `{min}-{max}` | Bound the contract price in ISK. |
| estimated-value | `estimated-value/{min}` or `{min}-{max}` | Bound the estimated value in ISK. |
| goldbar | flag | At least one attribute rolled the best value the abyssal type can reach. |
| brownbar | flag | At least one attribute rolled the worst value the type can reach. |
| diamondbar | flag | A gold bar rolled with a Glorified mutaplasmid. |
| item-exchange / auction | flag | Only contracts of that type. |
| no-multi-item-contracts | flag | Only contracts containing exactly one abyssal module and nothing else. |
| contracts-only | flag | Exclude modules listed only as MutaMarket sell listings. |
| without-other-items | flag | Only contracts that do not include unrelated items. |

Chained together:

```
GET /api/modules/type/abyssal-ballistic-control-system/sort/price/asc/goldbar/contract-price/0-500000000
```

## Polling for new listings

`sort/date-added/desc` orders by when a module's current contract was added
to MutaMarket. That order is append-only, so polling the first page shows
newly listed modules without walking every page.

## Identifying a module

A path segment ending in digits is a module lookup by EVE item id or
MutaMarket slug; anything else is the type-scoped list. Both live on the
same path.

```
GET /api/modules/1052842251186
GET /api/modules/abyssal-ballistic-control-system-1052842251186
```
