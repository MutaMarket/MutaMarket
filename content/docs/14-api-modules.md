---
section: API
---

# API: Modules

Browse abyssal modules that are for sale, retrieve a single module with all
rolled attributes and price data, and import modules from EVE.

## List modules of a type

```
GET /api/modules/{query}
```

Lists modules of an abyssal type that are currently for sale, either
through a public in-game contract or a MutaMarket sell listing. Newest
modules come first unless a sort option is given.

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
| goldbar | flag | Only modules with at least one attribute rolled better than the best regular meta variant. |
| brownbar | flag | Only modules with at least one attribute rolled worse than the worst regular meta variant. |
| diamondbar | flag | Only modules with the best recorded roll for the type in at least one attribute. |
| item-exchange / auction | flag | Only contracts of that type. |
| no-multi-item-contracts | flag | Only contracts containing exactly one abyssal module and nothing else. |
| contracts-only | flag | Exclude modules listed only as MutaMarket sell listings. |
| without-other-items | flag | Only contracts that do not include unrelated items. |

Full example:

```
GET /api/modules/type/abyssal-ballistic-control-system/sort/price/asc/goldbar/contract-price/0-500000000
```

**Query parameters**

| Name | Type | Meaning |
|---|---|---|
| `cursor` | string | Pagination cursor from `meta.next_cursor` of a previous response. |
| `region_id` | integer | Only modules whose contract is in this EVE region, e.g. `10000002` for The Forge. |

**Pagination.** Responses carry 100 modules per page. Request the next page
by passing `meta.next_cursor` as the `cursor` query parameter, or follow
`links.next`. Treat the cursor as opaque — its contents are not part of the
contract and have already changed once.

### Polling for new listings

`sort/date-added/desc` orders by when a module's current contract was added
to MutaMarket. That order is append-only, so polling the first page shows
newly listed modules without walking every page.

**Responses**

- `200` — a `data` array of modules, plus `links` and `meta`.
- `404` — `{"message": "Please provide a valid type."}` when the query names
  no valid abyssal type, including when the type segment is missing entirely.

## Get a single module

```
GET /api/modules/{module}
```

Returns one module with all rolled attributes, roll-quality metrics, the
estimated value, and its current sale listing (contract or MutaMarket sell
listing) if any.

The module must already be known to MutaMarket. If it is not, import it
first with `POST /api/modules`.

| Parameter | Meaning |
|---|---|
| `module` | The EVE item id, or the MutaMarket slug (`abyssal-ballistic-control-system-1052842251186`). |

A slug ending in digits is read as a module lookup; anything else is read
as the type-scoped list above, which is why both live on one path.

**Responses**

- `200` — `{"data": { ... }}`.
- `404` — `{"message": "No module with this item id is known to MutaMarket."}`

## Import a module from EVE

```
POST /api/modules
```

Imports a module into MutaMarket by its in-game item id and returns it with
all rolled attributes, roll-quality metrics and the estimated value.

Identify the module either explicitly with `type_id` + `item_id`, or by
pasting an in-game item link as `message` — any string containing
`showinfo:{type_id}//{item_id}` works, which is what you get when you drag
an item into the EVE chat window and copy the message.

**This is the expensive one.** The module data is fetched live from ESI and
the value estimation runs synchronously, so expect a few seconds per call.
Re-submitting an existing module refreshes it instead of duplicating it.
Please do not call it in a loop.

| Field | Type | Meaning |
|---|---|---|
| `message` | string | A string containing `showinfo:{type_id}//{item_id}`. Required when `type_id`/`item_id` are absent. |
| `type_id` | integer | The EVE type id of the mutated module, e.g. `47820`. Required with `item_id` when `message` is absent. |
| `item_id` | integer | The EVE item id of the module, e.g. `1041420958612`. Required with `type_id`. |

```json
{ "type_id": 47820, "item_id": 1041420958612 }
```

**Responses**

- `200` — `{"data": { ... }}`, the same shape as a single module.
- `400` — `{"message": "Failed to add module!"}` when the import failed, the
  ids do not name a real abyssal module, or a `message` carried no item link.
- `422` — a validation error naming the offending fields in `errors`.
