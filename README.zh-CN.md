# alpha-memory

[English](README.md) · **简体中文**

alpha-memory 是用 Rust 编写的记忆引擎，用于记录会话事件、保存任务检查点、检索原始证据，以及恢复工作所需的上下文。调用方通过命令行传入工作区和会话 ID。宿主插件和模型实验在各自的仓库维护。

基于 alpha-memory 的跨会话研究流程，是对论文 *A Proof of the Most Informative Boolean Function Conjecture* 中验证方法的一项实验性实践。论文把计算结果限定在明确的适用范围内，并说明了一个反例如何推翻某个推广命题，而原猜想仍有成立的可能。我们把这一区分用于记忆交接：接手的会话可以沿结论找到原始证据，判断哪些依赖结论已失去依据，并保留经过独立验证的结果。[论文原文（arXiv:2609.24931）](https://arxiv.org/abs/2609.24931)

## 调用方与模型

调用方决定何时读取历史，以及如何总结已经完成的工作。引擎本身无需调用模型。

- 调用方传入 `recall --open-long-term` 时，引擎会把有效的长期记忆加入召回结果。
- 调用方可以用 `reflect --body` 保存模型写好的摘要。省略正文时，引擎根据相应层级保存结构化信息，例如事件范围、类别计数、配置键、产物标识或中期条目 ID。

引擎在写入前对内容脱敏。会话日志只追加，长期条目可通过中期条目 ID 追溯到原始日志范围。配置值或同一主题的决策被替换时，相关旧条目会标记为已取代，原有记录仍可追溯。

## 记录与召回

### 写入会话日志

调用方提交的交互会追加到会话日志。每次写入若同时带有配置键和值，引擎也会更新配置表，保存该键的最新值及其来源事件。配置事件的主题默认采用配置键，后续修改可据此标记已过时的摘要。

引擎按以下优先级为事件分类，取第一项匹配的信号。

1. `--failure` 标记失败。
2. `--decision` 标记决策。
3. `--event-kind` 显式指定 `config`、`env`、`decision` 或 `failure`。
4. 同时提供配置键和值时，归为 `config`。
5. 提供 `--stage` 时，归为 `env`。

其余事件保留在日志中。每个事件最多归入一类。例如，带配置键值的失败事件归为失败，同时更新配置表。

### 按需召回会话记忆

`recall` 根据当前会话已有的记录组装上下文，包括：

- 用户目录中的长期约束和行为偏好。
- 会话中仍有效的中期条目。
- 尚未被中期条目覆盖的决策和失败事件。同一主题有多次决策时，保留最近一次。
- 最近若干事件中未出现的配置键及其最新值。默认窗口为 5 个事件，可用 `--recent` 调整。
- 当前阶段的事件，以及产物的路径、哈希和大小。

`--open-long-term` 还会纳入有效的长期条目和当前阶段较早的事件。用 `--stage` 可以指定阶段。近期出现过的配置键会从此次召回的配置列表中省略，其来源事件仍保存在日志里。

## 存储位置

| 记录 | 存储位置 | 作用范围 |
| --- | --- | --- |
| 行为偏好 | 用户目录 | 跨会话，每次召回时读取 |
| 长期约束 | 用户目录 | 跨会话，每次召回时读取 |
| 日志、配置、阶段、决策、失败、中期和长期条目、产物信息 | 工作区内的会话目录 | 当前会话 |

用户目录由 `$ALPHA_MEMORY_HOME` 指定，默认是 `~/.alpha-memory`。会话数据库位于 `<workspace>/.alpha-memory/sessions/<session-id>/store.sqlite`。再次传入相同的工作区和会话 ID 即可继续。需要跨会话使用的经验，可由调用方写成用户偏好或长期约束。

## 事件类别

| 类别 | 记录内容 | 召回行为 |
| --- | --- | --- |
| `config` | 端口、分支、测试命令、模型名称等精确值 | 配置表保留最新值，近期出现的键在召回时省略 |
| `env` | 会话的开发阶段 | 召回当前阶段，使用 `--open-long-term` 可纳入该阶段较早的事件 |
| `decision` | 后续工作需要遵循的选择 | 在中期条目覆盖前可见，同一主题的新决策取代旧决策 |
| `failure` | 一次失败的尝试 | 作为独立事件保存，在中期条目覆盖前直接召回 |

## 记忆整理

调用方可在会话结束、压缩上下文之前或任务切换时运行 `reflect`。

`reflect mid` 将一段连续日志整理为中期条目。`reflect long` 根据指定的中期条目 ID 写入长期条目，并将来源中期条目标记为已提升。默认召回会略去已提升的中期条目。调用方显式打开长期记忆后，即可读取对应的长期条目。

## 命令示例

以下示例在新会话中写入配置和决策，随后召回并整理记忆。

```sh
alpha-memory user standing --text "Do not run destructive commands without confirmation."
alpha-memory user preference --text "Prefer small patches."
alpha-memory append --session s1 --config-key port --config-value 4080
alpha-memory append --session s1 --decision --topic db --body "Use SQLite for the session log."
alpha-memory recall --session s1
alpha-memory recall --session s1 --open-long-term
alpha-memory reflect mid --session s1 --from 1 --to 2 --topic db --body "The session log is SQLite."
alpha-memory reflect long --session s1 --mids 1 --topic db --body "Keep the session log in SQLite."
```

`--body -` 从标准输入读取正文。`--workspace` 指定工作区，`--user-home` 指定本次调用使用的用户目录。

下面在新会话中将配置的来源事件固定到任务检查点，再分别查看任务上下文、查询精确配置值、搜索事件并读取原文。

```sh
alpha-memory append --session demo --config-key staging.probe.port --config-value 4761 --body "Legacy client requirement"
alpha-memory task --session demo --task probe --goal "Maintain staging probes" --pin 1 --hint "Check legacy client compatibility"
alpha-memory context --session demo
alpha-memory session-config --session demo --key staging.probe.port
alpha-memory index --session demo --query "legacy client"
alpha-memory source --session demo --event 1
```

`reflect pending` 只处理同一任务中尚未被覆盖的连续日志区间。已提升或已被取代的条目所覆盖的区间，也视为已覆盖。

使用 `cargo build --locked` 构建引擎，使用 `cargo test --locked` 运行引擎测试。

[结论版本、检查证据、多条论证与授权交接恢复](docs/claims.md)
