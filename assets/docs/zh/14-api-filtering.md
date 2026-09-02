---
section: API
---

# API：筛选与排序

`GET /api/modules/{query}` 以 URL 段而非查询参数的形式接收筛选和排序条件。本页是词汇表；请求和响应的结构见[端点参考](/documentation/api)。

## 选项

`query` 路径段**必须**包含一个类型选项（`type/{id-or-slug}`，例如 `type/49738` 或 `type/abyssal-ballistic-control-system`），并接受与模块浏览器相同的筛选选项，以 URL 段的形式按任意顺序串联。

| 选项 | 格式 | 效果 |
|---|---|---|
| sort | `sort/{field}/{direction}` | 按 `price`（合同价格）、`value`（估值）、`fraction`（平均突变质量）、`contract-date`（当前合同的发布时间）、`date-added`（模块当前合同加入 MutaMarket 的时间）排序，或按 id 或名称指定的 dogma 属性排序（`sort/50/desc`、`sort/cpu/asc`）。方向为 `asc` 或 `desc`。按属性排序只返回拥有该属性的模块。 |
| attributes | `attributes/{attribute}/{value}`（成对，可重复） | 按突变值筛选，例如 `attributes/cpu/20-30/damageMultiplier/2.1`。`min-max` 范围限定数值；单个数字在越高越好的属性上是最小值，否则是最大值。 |
| meta-group | `meta-group/{group}` | `t1`、`t2`、`storyline`、`faction`、`officer`、`deadspace` 之一：只返回由该元组的源模块突变而来的模块。 |
| meta-level | `meta-level/{n}` | 只返回由该元等级的源模块突变而来的模块。 |
| contract-price | `contract-price/{max}` 或 `{min}-{max}` | 限定合同价格，以 ISK 计。 |
| estimated-value | `estimated-value/{min}` 或 `{min}-{max}` | 限定估值，以 ISK 计。 |
| goldbar | 标志 | 至少有一个属性在 Unstable 突变质体上达到该类型的最佳可能值。 |
| brownbar | 标志 | 至少有一个属性在符合条件的突变质体上达到该类型的最差可能值。 |
| diamondbar | 标志 | 与 goldbar 相同，但在 Glorified 突变质体上获得。 |
| item-exchange / auction | 标志 | 只返回该类型的合同。 |
| no-multi-item-contracts | 标志 | 只返回恰好包含一个深渊模块且没有其他物品的合同。 |
| contracts-only | 标志 | 排除仅作为 MutaMarket 出售挂牌存在的模块。 |
| without-other-items | 标志 | 只返回不包含无关物品的合同。 |

串联起来：

```
GET /api/modules/type/abyssal-ballistic-control-system/sort/price/asc/goldbar/contract-price/0-500000000
```

## 轮询新挂牌

`sort/date-added/desc` 按模块当前合同加入 MutaMarket 的时间排序。该顺序是只追加的，所以轮询第一页就能看到新挂牌的模块，而无需遍历每一页。

## 标识模块

以数字结尾的路径段是按 EVE 物品 id 或 MutaMarket slug 查询单个模块；其他情况都是按类型范围的列表。两者位于同一路径。

```
GET /api/modules/1052842251186
GET /api/modules/abyssal-ballistic-control-system-1052842251186
```
