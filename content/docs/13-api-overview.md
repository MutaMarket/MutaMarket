---
section: API
---

# The MutaMarket API

MutaMarket has a public, read-mostly HTTP API for abyssal modules: browse
what is for sale, look up a single module with every rolled attribute and
its estimated value, import a module from EVE, and read the reference data
behind the roll-quality metrics.

It needs no key and no account. Everything is JSON, over HTTPS, at
`https://mutamarket.com/api`.

## Before you start

- `POST /api/modules` calls EVE's ESI and runs a price model on every
  request. It takes seconds. Do not call it in a tight loop.
- Send a User-Agent that identifies you, with a contact address.
- `/api/abyssal-type-statistics` changes a few times a year. Cache it.

## Conventions

Single objects come wrapped in a `data` key; the reference endpoints return
a bare array.

Errors are always a JSON object with a `message`, and the HTTP status
carries the meaning:

| Status | Meaning |
|---|---|
| 400 | The request was understood but could not be acted on. |
| 404 | No such module, or the query named no valid abyssal type. |
| 422 | The request was well-formed but a value was not acceptable. `errors` names the fields. |
| 500 | Our fault. |

```json
{ "message": "Please provide a valid type." }
```

## Which endpoints are stable

Only the endpoints documented in this section are public, and only they
carry a compatibility promise: we will not remove a field or change its
meaning without notice.

Everything else under `/api` serves mutamarket.com itself and changes
without warning. If you need something that is only available there, ask
for it rather than depending on it.

## Reference

The [endpoint reference](/documentation/api) lists every endpoint with its
parameters, responses and schemas.
[`/api/openapi.json`](/api/openapi.json) is the same description in
machine-readable form.

## Getting in touch

Bugs, missing data, or an endpoint you wish existed: see
[Support](/documentation/support).
