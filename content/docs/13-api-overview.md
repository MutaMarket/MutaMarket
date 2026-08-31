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

- **Be reasonable.** There is no hard rate limit, and we would rather not
  add one. `POST /api/modules` in particular calls EVE's ESI and runs a
  price model on every request, so it takes seconds and costs us a slice
  of our ESI error budget. Do not call it in a tight loop.
- **Send a User-Agent that identifies you**, ideally with a contact. It is
  the difference between us emailing you about a problem and us blocking
  you. CCP asks the same of everyone using ESI.
- **Cache what does not move.** `/api/abyssal-type-statistics` changes when
  a new abyssal type ships, which is a few times a year.

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

Everything else under `/api` exists to serve mutamarket.com itself. Those
paths change whenever the site changes, without warning, and are not
documented here. If you find yourself reading `/api/module-page/...` or
`/api/sidebar`, you are on a path that will break — tell us what you needed
and we will look at exposing it properly.

## Machine-readable

An OpenAPI description of everything in this section lives at
[`/api/openapi.json`](/api/openapi.json). It is generated from the server's
own route definitions and response types, so it describes what the code
actually returns rather than what someone remembered to write down.

## Getting in touch

Bugs, missing data, or an endpoint you wish existed: see
[Support](/documentation/support). Tell us what you are building — it makes
the difference between a guess and a good endpoint.
