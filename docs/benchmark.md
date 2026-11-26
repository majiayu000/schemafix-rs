# 基准测试：修复效果与性能

本文只写实测数字。原始结果在 `bench/results/{before,after,after-rerun}/`，包括每个工具每条输出（`*.jsonl`）、每次调用的耗时样本（`*.timing.json`）、汇总（`summary.json`）和机器信息（`env.json`）。

## 复现

```bash
bench/run.sh after 5
```

这条命令依次做：`uv sync`（Python 依赖）、`npm ci`（Node 依赖，装在 `bench/node/` 本地）、`cargo build --release`（`bench/rust`），用固定种子生成语料，在同一份语料上跑全部 10 个工具配置，最后用同一个裁判打分。第二个参数是重复轮数。

## 环境（`bench/results/after/env.json`）

| 项 | 值 |
|---|---|
| 机器 | MacBookPro18,3，Apple M1 Pro，8 核，32 GB |
| 系统 | Darwin 25.6.0 |
| Rust | rustc 1.100.0，release 构建 |
| Python | 3.14.0（uv 管理） |
| Node | v22.18.0 |
| 竞品版本 | json-repair 0.64.9、jsonschema(py) 4.26.0、instructor 1.17.0、pydantic 2.14.4、jsonrepair(js) 3.15.0、ajv 8.20.0、rstructor 0.6.0、jsonrepair-rs 0.2.7、jsonschema(rs) 0.63.1 |

## 语料

- **全部是合成数据。** 生成器：`bench/corpus/{truth,corrupt,traps,gen}.py`；种子 `20261126`；每个 schema 30 个随机基础实例。共 **2070 条**（syntax 900、schema 750、trap 420），SHA-1 为 `7310300595db10246c8f94cdc8fa10ff3b949794`。
- schema 有 3 个，在 `bench/schemas/`：
  - `weather`：enum、integer 范围、boolean default。
  - `order`：嵌套对象、对象数组、enum default、nullable，各层都是 `additionalProperties:false`。
  - `event`：`$defs` + `$ref`、`anyOf`、`["integer","null"]`。
- 每条语料是"一个合法实例 + 一种损坏"，共 28 类：
  - **syntax**（10 类，期望值是原实例）：clean、code_fence、prose_fence、prose_inline、trailing_comma、single_quotes、unquoted_keys、python_literals、truncated、comments。
  - **schema**（12 类，期望值是原实例）：int_as_string、number_as_string、bool_as_string、enum_case、singleton_array、null_as_string、missing_default、extra_property、number_for_string、anyof_branch、stringified_json、combined（说明文字 + 代码块 + 数字字符串 + enum 全大写）。
  - **trap**（6 类）：fractional_int（`"2.5"` 给 integer）、missing_required（缺少没有 default 的必填字段）、unfixable_type（`"next week"` 给 integer）、enum_unknown、precision_float（`"19.990000000000000000001"`）这 5 类，期望结果是"不产出合法值"。precision_int（23 位整数字符串）期望转成精确整数，因为 Python 能表示。
- 期望值有争议的情况不收录：`order.notes` 的类型是 `["string","null"]`，字符串 `"null"` 本身就合法，所以这个字段没放进 null_as_string。
- **真实 LLM 坏输出：没有。** 没找到许可明确、内容确认是真实模型坏输出的公开数据集。检索到的候选：
  - `ottema/structfix-bench`：页面写明是合成损坏，许可未核实。
  - `minpeter/hermes-function-calling-v1-parsed` 的 `func-calling-error.jsonl`：页面标注 apache-2.0，但内容是否为模型坏输出未核实。
  - 所以本报告没有"真实数据"这一组。
- **偏差说明（推断）：** 损坏类别是我设计的，其中 enum_case、null_as_string 这些是 schemafix 实现了的纠正。json_repair 的 yes/no 转布尔、浮点转字符串、salvage 列表转对象等能力不在语料里。结果代表"这 28 类问题"上的表现，不代表任意真实输出。

## 指标定义（`bench/score.py`）

- **裁判**：所有工具的输出统一用 Python jsonschema（Draft 2020-12）重新校验，不采信工具自己的判断。
- **修复后通过率** = 产出了通过裁判校验的值的条数 ÷ 期望有值的条数（1680）。
- **正确率** = 正确的条数 ÷ 全部 2070 条。期望有值的：输出与期望值严格相等才算对（布尔和数字不混同，3 和 3.0 视为相等）。trap 类：没有产出合法值才算对。
- **错误修复率** = 产出了通过校验、但和期望值不一致的条数（包括 trap 被"修"成合法值）÷ 2070。
- **性能**：单线程。先完整跑一遍预热，再跑 5 轮，每次调用单独计时（Rust 用 `Instant`，Python 用 `perf_counter_ns`，Node 用 `hrtime.bigint`）。
  - 计时覆盖"原始文本 → 结果值"整个调用；schema、校验器、pydantic 模型都在计时外预先构建好。
  - 次/秒 = 调用数 ÷ 耗时总和；MB/s = 输入字节 × 轮数 ÷ 耗时总和。

## 结果：全部工具（`after`，即优化后）

| 工具 / 配置 | 版本 | 修复后通过率 | 正确率 | 错误修复 | 次/秒 | MB/s | p50 µs | p99 µs | 复跑次/秒 |
|---|---|---|---|---|---|---|---|---|---|
| **schemafix**（drop_additional=true） | schemafix-rs 0.1.0 | **1650/1680（98.2%）** | **2040/2070（98.6%）** | **0（0.0%）** | 236,285 | 46.74 | 4.0 | 9.2 | 244,322 |
| json_repair schema=standard | 0.64.9 | 1410/1680（83.9%） | 1770/2070（85.5%） | 30（1.4%） | 4,658 | 0.92 | 176.3 | 825.3 | 4,532 |
| json_repair schema=salvage | 0.64.9 | 1530/1680（91.1%） | 1756/2070（84.8%） | 164（7.9%） | 4,763 | 0.94 | 184.8 | 635.5 | 4,668 |
| json_repair（无 schema）+ jsonschema | 0.64.9 + 4.26.0 | 900/1680（53.6%） | 1290/2070（62.3%） | 0（0.0%） | 17,032 | 3.37 | 42.2 | 233.8 | 16,362 |
| instructor 校验层（pydantic 宽松模式） | 1.17.0 + 2.14.4 | 771/1680（45.9%） | 1131/2070（54.6%） | 30（1.4%） | 81,465 | 16.11 | 12.2 | 27.6 | 82,076 |
| jsonrepair + ajv（coerce/defaults/removeAdditional） | 3.15.0 + 8.20.0 | 1170/1680（69.6%） | 1500/2070（72.5%） | 60（2.9%） | 107,808 | 21.32 | 8.8 | 21.4 | 103,926 |
| jsonrepair + ajv（不纠正） | 3.15.0 + 8.20.0 | 720/1680（42.9%） | 1110/2070（53.6%） | 0（0.0%） | 109,286 | 21.62 | 8.5 | 22.9 | 108,510 |
| jsonrepair-rs + jsonschema（不纠正，对照） | 0.2.7 + 0.63.1 | 720/1680（42.9%） | 1110/2070（53.6%） | 0（0.0%） | 431,296 | 85.31 | 2.3 | 4.8 | 444,097 |
| rstructor MockClient 完整 materialize | 0.6.0 | 201/1680（12.0%） | 591/2070（28.6%） | 0（0.0%） | 70,463 | 13.94 | 15.8 | 22.7 | 69,313 |
| rstructor 解码步骤（serde_path_to_error + validate） | 0.6.0 | 201/1680（12.0%） | 591/2070（28.6%） | 0（0.0%） | 1,179,671 | 233.33 | 0.7 | 2.8 | 957,655 |

"复跑"列来自 `bench/results/after-rerun/`，两次运行的正确率完全相同，吞吐差异见该列。

### 分组延迟 p50 / p99（µs，`after`）

| 工具 | syntax | schema | trap |
|---|---|---|---|
| schemafix | 2.2 / 7.8 | 5.1 / 9.5 | 5.8 / 10.2 |
| json_repair standard | 177.5 / 855.6 | 176.7 / 798.8 | 175.4 / 786.2 |
| json_repair salvage | 208.2 / 702.6 | 178.9 / 616.2 | 176.5 / 635.5 |
| json_repair 无 schema | 70.1 / 255.4 | 29.5 / 96.0 | 29.7 / 94.5 |
| instructor 校验层 | 13.0 / 30.8 | 11.8 / 25.5 | 12.0 / 20.2 |
| ajv coerce | 8.8 / 24.5 | 8.5 / 20.7 | 9.2 / 16.4 |
| ajv 不纠正 | 8.7 / 23.3 | 8.3 / 23.7 | 8.8 / 20.0 |
| jsonrepair-rs + jsonschema | 2.4 / 4.9 | 2.3 / 4.5 | 2.4 / 4.0 |
| rstructor MockClient | 14.8 / 22.2 | 16.2 / 24.2 | 15.8 / 22.4 |
| rstructor 解码 | 0.2 / 2.9 | 1.0 / 2.8 | 1.0 / 2.1 |

## schemafix 优化前后（同一份语料、同一台机器）

`before` 是提交 7477e08 的代码，`after` 是本轮改动后的代码。

| 指标 | before | after | 复跑 |
|---|---|---|---|
| 修复后通过率 | 1260/1680（75.0%） | 1650/1680（98.2%） | 1650/1680 |
| 正确率 | 1650/2070（79.7%） | 2040/2070（98.6%） | 2040/2070 |
| 错误修复 | 0 | 0 | 0 |
| 次/秒 | 139,012 | 236,285 | 244,322 |
| MB/s | 27.50 | 46.74 | 48.33 |
| p50 / p99 µs | 4.3 / 24.6 | 4.0 / 9.2 | 3.9 / 8.9 |
| schema 组 p50 / p99 µs | 6.4 / 27.1 | 5.1 / 9.5 | — |

从 before 到 after，各类别的变化：

| 类别 | before | after | 对应改动 |
|---|---|---|---|
| prose_fence | 0/90 | 90/90 | 新增从说明文字/代码块中提取 JSON（`src/parse.rs`） |
| prose_inline | 0/90 | 90/90 | 同上 |
| combined | 0/90 | 90/90 | 同上 |
| number_for_string | 0/60 | 60/60 | 整数 → 字符串 |
| stringified_json | 0/60 | 60/60 | 字符串化的 JSON 解包 |

性能改动：
- anyOf/oneOf 的分支校验器改为在 `Fixer::new` 时预编译缓存，原来每次调用都重新编译。
- 本来就合法的输入只校验一次，原来是 `is_valid` 之后又跑一遍 `iter_errors`。

本轮还新增了 allOf 纠正、`"3.0"` → 3、单元素数组拆包，都有测试，但语料里没有对应类别，所以上面的数字不反映它们。

开发中出过一次回归：第一版提取逻辑取"最后一个 `}`"，把 truncated 类从 90/90 打到 30/90。改为考虑字符串的括号配对后恢复 90/90，并加了回归测试（`tests/extract.rs::truncated_nested_output_is_not_cut_short`、`src/parse.rs` 单测）。那次中间运行的原始文件没有保留。

## 各类别正确数（`after`）

| 类别 | schemafix | json_repair std | json_repair salvage | instructor | ajv coerce | jsonrepair-rs+jsonschema | rstructor |
|---|---|---|---|---|---|---|---|
| syntax/clean | 90/90 | 90/90 | 90/90 | 90/90 | 90/90 | 90/90 | 90/90 |
| syntax/code_fence | 90/90 | 90/90 | 90/90 | 90/90 | 90/90 | 90/90 | 0/90 |
| syntax/prose_fence | 90/90 | 90/90 | 90/90 | 90/90 | 0/90 | 0/90 | 0/90 |
| syntax/prose_inline | 90/90 | 90/90 | 90/90 | 90/90 | 0/90 | 0/90 | 0/90 |
| syntax/trailing_comma | 90/90 | 90/90 | 90/90 | 0/90 | 90/90 | 90/90 | 0/90 |
| syntax/single_quotes | 90/90 | 90/90 | 90/90 | 0/90 | 90/90 | 90/90 | 0/90 |
| syntax/unquoted_keys | 90/90 | 90/90 | 90/90 | 0/90 | 90/90 | 90/90 | 0/90 |
| syntax/python_literals | 90/90 | 90/90 | 90/90 | 21/90 | 90/90 | 90/90 | 21/90 |
| syntax/truncated | 90/90 | 90/90 | 90/90 | 0/90 | 90/90 | 90/90 | 0/90 |
| syntax/comments | 90/90 | 90/90 | 90/90 | 0/90 | 90/90 | 90/90 | 0/90 |
| schema/int_as_string | 90/90 | 90/90 | 90/90 | 90/90 | 90/90 | 0/90 | 0/90 |
| schema/number_as_string | 30/30 | 30/30 | 30/30 | 30/30 | 30/30 | 0/30 | 0/30 |
| schema/bool_as_string | 60/60 | 60/60 | 60/60 | 60/60 | 60/60 | 0/60 | 0/60 |
| schema/enum_case | 90/90 | 0/90 | 0/90（错 30） | 0/90 | 0/90 | 0/90 | 0/90 |
| schema/singleton_array | 60/60 | 60/60 | 60/60 | 0/60 | 0/60 | 0/60 | 0/60 |
| schema/null_as_string | 30/30 | 0/30 | 0/30 | 0/30 | 0/30 | 0/30 | 0/30 |
| schema/missing_default | 60/60 | 0/60 | 60/60 | 60/60 | 60/60 | 0/60 | 0/60 |
| schema/extra_property | 90/90 | 90/90 | 90/90 | 90/90 | 90/90 | 0/90 | 90/90 |
| schema/number_for_string | 60/60 | 60/60 | 60/60 | 0/60 | 60/60 | 0/60 | 0/60 |
| schema/anyof_branch | 30/30 | 30/30 | 30/30 | 30/30 | 30/30 | 0/30 | 0/30 |
| schema/stringified_json | 60/60 | 60/60 | 60/60 | 0/60 | 0/60 | 0/60 | 0/60 |
| schema/combined | 90/90 | 0/90 | 0/90（错 30） | 0/90 | 0/90 | 0/90 | 0/90 |
| trap/fractional_int | 90/90 | 90/90 | 68/90（错 22） | 90/90 | 90/90 | 90/90 | 90/90 |
| trap/missing_required | 90/90 | 90/90 | 90/90 | 90/90 | 90/90 | 90/90 | 90/90 |
| trap/unfixable_type | 90/90 | 90/90 | 68/90（错 22） | 90/90 | 90/90 | 90/90 | 90/90 |
| trap/enum_unknown | 90/90 | 90/90 | 60/90（错 30） | 90/90 | 90/90 | 90/90 | 90/90 |
| trap/precision_float | 30/30 | 0/30（错 30） | 0/30（错 30） | 0/30（错 30） | 0/30（错 30） | 30/30 | 30/30 |
| trap/precision_int | **0/30** | 30/30 | 30/30 | 30/30 | 0/30（错 30） | 0/30 | 0/30 |

（rstructor 一栏是 `rstructor_decode`；`rstructor_mock` 各类别结果与它完全相同。）

## 测量口径说明（事实）

- json_repair 带 schema 调用时，每次调用都在函数内部新建 `SchemaRepairer`（`json_repair.py:161-166`），它的公开 API 没有可复用的预编译对象，所以这份开销算在它的耗时里。
- `rstructor_mock` 每次调用都会生成 schema、记录请求（`backend/mock.rs:738-748`）。它的耗时包括这些 mock 簿记，不等于真实解析开销；`rstructor_decode` 只复刻它的解码步骤，更接近纯解析开销。rstructor 快，是因为它遇到任何偏差都直接失败，不做修复。
- Python 和 Node 的耗时包括解释器本身的调用开销；全部单线程，没有测并发。
- 两次运行之间吞吐波动：schemafix +3.4%、ajv coerce −3.6%、rstructor_decode −18.8%，其余在 ±5% 以内。
