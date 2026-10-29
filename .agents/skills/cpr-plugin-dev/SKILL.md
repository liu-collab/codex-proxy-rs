---
name: cpr-plugin-dev
description: 开发、修复、排查、审查或打包 Codex Proxy RS 独立网关插件，含处理器、清单与插件页面；仅改宿主能力时使用 cpr-dev-guide 的插件边界，不适用于 Codex 插件或 Vue 插件
---

# 网关插件开发

## 先读职责边界

插件在独立项目中通过公开 SDK 实现业务，插件业务错误在插件侧修复；宿主维护自身合同、资源与稳定性，不猜测插件意图、不补造业务数据，不在宿主、SDK 或 Runtime 中增加插件专属补偿

开发、排查和审查先完整读取[插件职责边界](../cpr-dev-guide/references/plugin-boundaries.md)，再选择实现步骤；已有处理器修复和页面改动也执行，不以是否新增能力为条件。需要新增宿主能力时，先界定可独立使用的基础接口及其职责；必要的协议保护与资源回收仍由宿主负责

同时执行 [cpr-dev-guide](../cpr-dev-guide/SKILL.md)的领域必读与收尾要求；已实际读取且仍适用的内容复用，不递归重读入口。仅构建、打包或分发已有插件时按对应操作读取，若需诊断或修改行为，再补齐开发分支

## 范围与合同

- 确认用途、工程位置、目标宿主版本与平台，沿用已有工程；位置无法推断时只补齐该信息，不把插件源码塞进宿主 workspace
- 核对运行宿主、[SDK 版本](../../../backend/crates/gateway-plugin/sdk/Cargo.toml)、[宿主支持清单](../../../backend/crates/gateway-plugin/runtime/plugin-host-compatibility.json)及所用 CLI，清单、进程和能力版本分别判断
- `contributes` 声明处理器，`bindings` 选择挂载与匹配范围；安装成功不等于功能生效。当前清单拒绝 `permissions`，安装者完整信任插件，`trustedProcess` 不提供操作系统沙箱或访问域隔离，见[完整信任](../../../backend/crates/gateway-plugin/sdk/docs/manifest.md#完整信任)
- Codex 兼容性适配按[问题与方案依据](../../../CONTRIBUTING.md#问题与方案依据)执行，不因位于插件中扩大改写或补偿范围；业务改写限于用户授权，宿主能力以目标 SDK 为准

## 按当前步骤读取

完成上述适用边界后，按当前步骤读取下表；同时涉及多项时叠加。命中的实现或页面参考完整读取，长文档读取指定章节，示例、前端和打包资料在对应步骤加载

| 当前工作 | 入口 |
| --- | --- |
| 选择、使用或审查能力 | [能力与方法](../../../backend/crates/gateway-plugin/sdk/docs/capabilities.md#能力与方法)及所用能力章节，不以已有调用能编译代替合同核对 |
| 开发、排查或审查清单与 Rust 处理器 | [实现参考](references/implementation.md)、所用 SDK 类型；涉及清单字段时读[清单](../../../backend/crates/gateway-plugin/sdk/docs/manifest.md)对应章节 |
| 创建、修改或审查页面 | [页面参考](references/frontend.md)，不需要页面的插件不创建前端 |
| 准备工程或联调依赖 | [工程与依赖](references/development.md#工程与依赖)，需要示例时再读[复用示例](references/development.md#复用示例) |
| 构建或打包 | [构建与打包](references/development.md#构建与打包) |
| 实际验收 | [验证与交付](references/development.md#验证与交付)，按实现范围验证 |
| 分发插件 | [发布与安装来源](references/development.md#发布与安装来源)，不自动进入宿主发版流程 |
| 修改插件文档 | [文档检查](../cpr-dev-guide/references/documentation.md) |

## 交付

说明源码与产物位置、兼容范围、能力与绑定、实际验证及缺口，只报告真正生成的安装包或完成的业务验证

修改源码不默认授权在目标实例安装启用、调用真实上游或发布，沿用用户当前授权判断后续动作

PR 任务才使用 [cpr-github-pr](../cpr-github-pr/SKILL.md)，文档检查按开发入口收尾执行
