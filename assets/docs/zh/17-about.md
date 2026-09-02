---
section: 综合
---

# 关于

我是 Nicolas Kion，MutaMarket 是我做的。

起因是我每天都遇到同样的问题：深渊模块散落在各个角色和容器里，不知道它们值多少，也没有好办法找到我想要的特定结果。游戏内的工具对这些都帮不上忙。

所以 MutaMarket 就是我希望存在的东西。你从 EVE 导入模块，它根据相似结果的实际成交价告诉你它们值多少，并给你一个出售它们的地方和一个记录自己所有物的地方。

## 技术构成

后端是 Rust，使用 Axum 和 Postgres。前端是 SvelteKit 配合 Tailwind。市场和资产数据来自 EVE 的 ESI API。

价格估值来自按模块类型分别用真实成交记录训练的随机森林。[估价](/documentation/appraisal)说明了它的原理和失准之处。

## 联系方式

Abyssal Trading Discord 是交易社区所在的地方，我也在那里。MutaMarket 开发 Discord 是提交 bug 和功能请求的地方。两者都在页脚有链接。

你也可以在游戏中给 Nicolas Kion 发邮件，或发送电子邮件至 [nicolaskion07@gmail.com](mailto:nicolaskion07@gmail.com)。
