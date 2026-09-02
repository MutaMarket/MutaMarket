---
section: API
---

# MutaMarket API

MutaMarket 提供一个公开的、以读取为主的深渊模块 HTTP API：浏览在售模块，查询单个模块及其全部突变属性和估值，从 EVE 导入模块，以及读取突变质量指标背后的参考数据。

它不需要密钥，也不需要账户。一切都是 JSON，通过 HTTPS，位于 `https://mutamarket.com/api`。

## 开始之前

- `POST /api/modules` 会在每次请求时调用 EVE 的 ESI 并运行价格模型。这需要几秒钟。不要在紧密循环中调用它。
- 发送一个能标识你身份的 User-Agent，并附上联系方式。
- `/api/abyssal-type-statistics` 一年只变化几次。请缓存它。

## 约定

单个对象包裹在 `data` 键中；参考数据端点返回裸数组。

错误始终是一个带有 `message` 的 JSON 对象，HTTP 状态码表达含义：

| 状态码 | 含义 |
|---|---|
| 400 | 请求可以理解，但无法执行。 |
| 404 | 没有这样的模块，或查询未指定有效的深渊类型。 |
| 422 | 请求格式正确，但某个值不可接受。`errors` 指明相关字段。 |
| 500 | 我们的问题。 |

```json
{ "message": "Please provide a valid type." }
```

## 哪些端点是稳定的

只有本节记录的端点是公开的，也只有它们带有兼容性承诺：我们不会在没有通知的情况下移除字段或更改其含义。

`/api` 下的其他一切服务于 mutamarket.com 自身，会在没有预警的情况下更改。如果你需要的东西只在那里可用，请向我们提出，而不是依赖它。

## 参考

[端点参考](/documentation/api)列出每个端点及其参数、响应和模式。[`/api/openapi.json`](/api/openapi.json) 是同一描述的机器可读形式。

## 联系我们

Bug、缺失的数据，或你希望存在的端点：见[支持](/documentation/support)。
