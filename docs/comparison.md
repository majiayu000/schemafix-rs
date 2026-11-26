# schemafix-rs 与同类工具功能对比

日期：2026-11-26。被比较的版本即 `bench/` 里实际安装、实际运行的版本。

图例：**有** / **无** / **部分** / **未核实**（没查到证据，不猜）/ **不适用**。
"实测"指 `bench/run.sh` 跑出来的结果（`bench/results/after/summary.json`），或下文"手工核实命令"里列出的命令。
源码位置写的是安装后的包路径和行号（json_repair 0.64.9、instructor 1.17.0、rstructor 0.6.0）。

## 1. 被比较的对象

| 对象 | 版本 | 怎么跑的 | 来源 |
|---|---|---|---|
| schemafix-rs（本库） | 0.1.0（工作区） | `Fixer::with_options`，`drop_additional_properties=true` | 本仓库 |
| rstructor（Rust） | 0.6.0 | `MockClient` 离线跑完整 `materialize`（retries=0）；另单测其解码步骤 | <https://github.com/clifton/rstructor>，crates.io |
| json_repair（Python）+ jsonschema | 0.64.9 + 4.26.0 | `json_repair.loads(raw, schema=…)`，分 `standard` / `salvage` 两种模式；另测不带 schema 的版本加 jsonschema 校验 | <https://github.com/mangiucugna/json_repair>，PyPI |
| instructor（校验层） | 1.17.0 + pydantic 2.14.4 | 只跑它的 JSON 模式解析层：`extract_json_from_codeblock` → `json.loads(strict=False)` → pydantic 宽松模式 `model_validate` | `instructor/v2/core/json.py:13`，`instructor/v2/core/function_calls.py:85-110` |
| jsonrepair（JS）+ ajv | 3.15.0 + 8.20.0 | `jsonrepair` → `JSON.parse` → ajv 2020；分不带纠正，以及 `coerceTypes:"array", useDefaults, removeAdditional` 两种配置 | <https://github.com/josdejong/jsonrepair>，<https://ajv.js.org/coercion.html>，<https://ajv.js.org/guide/modifying-data.html> |
| jsonrepair-rs + jsonschema（Rust） | 0.2.7 + 0.63.1 | 只做语法修复和校验，不做 schema 纠正（本库的下层能力，作为对照） | crates.io |
| BAML（Schema-Aligned Parsing） | — | **未运行**：要用 BAML DSL 和代码生成，没法和上面几个共用同一份 JSON Schema 语料 | <https://www.boundaryml.com/blog/schema-aligned-parsing> |

BAML 的能力只来自它官方博客的说法，**全部未核实**，下表不单独给它列一栏。

## 2. 定位内功能对比

| 功能 | schemafix-rs | rstructor | json_repair | instructor 校验层 | jsonrepair+ajv | 依据 |
|---|---|---|---|---|---|---|
| 语法修复（尾逗号、单引号、无引号键、注释、截断、Python 字面量） | 有 | 无 | 有 | 无 | 有 | 实测：syntax/* 各类，rstructor、instructor 是 0/90（python_literals 只有不含布尔/null 的 21 条能过） |
| 代码块包裹 | 有 | 无 | 有 | 有 | 有 | 实测 syntax/code_fence |
| 前后带说明文字的 JSON | 有（本轮新增；取第一个括号配对完整的值） | 无 | 有 | 有（取**最后一个** JSON 片段） | 无 | 实测 prose_fence / prose_inline：ajv 组合 0/90；instructor `json.py:13` 的注释写着取最后一个 |
| 字符串 → 数字/整数 | 有 | 无 | 有 | 有 | 有 | 实测 int_as_string / number_as_string |
| `"3.0"` → 3（整数字段） | 有（本轮新增，仅 \|n\| ≤ 2^53 且精确） | 无 | 有 | 未核实 | 有 | 手工核实命令 1、2 |
| 丢精度时拒绝转换 | 有 | 不适用（不转换） | 无 | 无 | 无 | 实测 trap/precision_float：json_repair、instructor、ajv 都是 30/30 错误修复；手工命令 1 里 `"1e30"` 被 json_repair 转成 1000000000000000019884624838656 |
| 超出 i64/u64 的大整数字符串按原样保住 | **无**（拒绝，不错修） | 无 | 有（Python int 精度无限） | 有 | 无（转成丢精度的 float，实测错 30） | 实测 trap/precision_int |
| 字符串 → 布尔 | 部分（只认 `true`/`false`，不区分大小写） | 无 | 有（还接受 yes/no/on/off/1/0） | 有（pydantic 宽松模式） | 部分（`"true"`/`"false"`，`"yes"` 不行） | `schema_repair.py:779-786`；手工命令 2 |
| `"null"` → null（schema 允许 null 且不允许 string） | 有 | 无 | 无 | 无 | 无（ajv 只把 `""` 转 null） | 实测 schema/null_as_string 都是 0/30；手工命令 2 |
| 数字 → 字符串 | 部分（只转整数，本轮新增） | 无 | 有（含浮点） | 无 | 有（含浮点） | 实测 number_for_string：instructor 0/60；手工命令 1、2 |
| enum 大小写纠正 | 有（只在唯一匹配时） | 无 | 无（salvage 模式会填第一个 enum 值，实测错 30） | 无 | 无 | 实测 schema/enum_case |
| 单值包成数组 | 有 | 无 | 有 | 无 | 部分（只包标量，不包对象） | 实测 singleton_array：ajv 0/60 |
| 单元素数组拆包成标量 | 有（本轮新增） | 未核实 | 无 | 未核实 | 有 | 手工命令 1（json_repair 报错）、2（ajv 能过） |
| 字符串化的 JSON 对象/数组解包 | 有（本轮新增） | 无 | 有 | 无 | 无 | 实测 stringified_json；`schema_repair.py:414` 的 `_load_json_string_container` |
| required 缺失时补 `default` | 有 | 无 | 部分（standard 直接抛错，salvage 会补） | 有（pydantic 默认值） | 有（`useDefaults`） | 实测 missing_default：json_repair standard 0/60；`schema_repair.py:562-564` |
| 非 required 属性补 `default` | 无（设计上不改动合法值） | 无 | 有 | 有 | 有 | `schema_repair.py:572-574`；ajv 文档 modifying-data |
| `additionalProperties:false` 时删多余键 | 有（可配置，默认关） | 有（serde 忽略未知字段） | 有（总是删） | 有（pydantic 默认忽略） | 有（`removeAdditional`） | 实测 extra_property |
| 结合 `patternProperties` 判断多余键 | 无（存在 patternProperties 时一律不删） | 不适用 | 有（正则匹配） | 不适用 | 未核实 | `utils/pattern_properties.py:7` |
| anyOf / oneOf 选分支 | 有（oneOf 还要求只匹配一个分支） | 无 | 有（取第一个能过的，不检查 oneOf 唯一性） | 有（pydantic Union） | 有 | 实测 anyof_branch；`schema_repair.py:370-382` |
| allOf | 有（本轮新增，依次套用每个子 schema 的纠正） | 不适用 | 有 | 不适用 | 未核实 | `schema_repair.py:335-342`；本库测试 `tests/combinators.rs` |
| 本地 `$ref` | 有 | 不适用 | 有（只支持 `#/`） | 不适用 | 有 | `schema_repair.py:805-822` |
| 远程 `$ref` | 无 | 不适用 | 无（报 Unsupported） | 不适用 | 未核实 | 同上 |
| 完整 JSON Schema 校验 | 有（jsonschema crate） | 无（serde 类型 + 自定义 `validate`） | 有（依赖 Python jsonschema） | 无（pydantic 模型，不是 JSON Schema） | 有 | 各自源码 |
| 结构化修复记录 | 有（`Fix{path, kind, before, after}`） | 无 | 部分（`logging=True` 给出文字和路径，没有前后值） | 无 | 无 | 手工命令 1 的输出 |
| 能回喂给 LLM 的错误文本 | 有（`retry_hint()`，列出全部错误和 JSON Pointer） | 有（内部用于重试） | 部分（一条 ValueError 消息） | 有（重试循环内部的 reask） | 部分（`errors` 数组，要自己渲染） | rstructor `backend/messages.rs:164`（ValidationFailureContext）；instructor `v2/core/handler.py:44` |
| 有歧义时保持原值 | 有（trap 类错误修复 0 条） | 有（不纠正） | 部分（standard 错 30 条；salvage 错 164 条） | 部分（错 30 条） | 部分（错 60 条） | 实测 wrong_fix |
| 类型化解析 | 有（feature `schemars`） | 有 | 部分（可以传 pydantic 模型当 schema，返回的是 dict） | 有 | 无 | 各自文档和源码 |
| CLI | 有（退出码 0/1/2/3） | 无 | 有（`json_repair --schema`） | 不适用 | 未核实 | `uv run json_repair --help` |
| 流式增量修复 | 无（spec 本期不做） | 定位外（有流式对象，属于 LLM 调用侧） | 有（`stream_stable`） | 定位外 | 有（`jsonrepair/stream`，只修语法） | `json_repair.py:44`；`node_modules/jsonrepair/package.json` 的 exports |
| 输入大小上限 | 有（`max_input_bytes`） | 未核实 | 未核实 | 有（提取时 1 MiB 上限） | 未核实 | `instructor/v2/core/json.py:9` |

## 3. 定位外（列出来，不算作要超越的项）

| 能力 | 谁有 | 说明 |
|---|---|---|
| 调用 LLM、支持多家 provider | rstructor、instructor、BAML | schemafix 不调用模型 |
| 重试 / reask 循环 | rstructor（`backend/client.rs:287`，默认 3 次）、instructor（`v2/core/patch.py:51` 的 `max_retries`） | schemafix 只给 `retry_hint()`，循环由调用方自己写 |
| 约束解码 / 语法导出 | outlines、llguidance | spec 列为 v0.2 候选 |
| Prompt DSL 和代码生成 | BAML | — |
| 业务校验器（pydantic validator、rstructor `Instructor::validate`） | instructor、rstructor | 调用方可以在 `parse::<T>` 之后自己校验 |

## 4. 结论

【事实，来自本次实测】
- 实测的 28 个语料类别里，schemafix 在 27 个类别全对，错误修复为 0。
- 唯一没过的是 `trap/precision_int`（超出 u64 的整数字符串）：schemafix 选择拒绝，不错修；json_repair 和 instructor 借助 Python 的无限精度整数能正确转换。

【仍然落后或没做的功能（定位内）】
1. 超大整数精确保留：需要打开 serde_json 的 `arbitrary_precision` 和 jsonschema 的 `arbitrary-precision`。Cargo feature 会在整个依赖图里统一生效，下游所有 serde_json 的 Number 表示都会跟着变。**没做，等用户决定。**
2. 非 required 属性补 default：会改动本来合法的值，和"合法值不动"的设计冲突。**没做，等用户决定。**
3. 识别 `patternProperties` 再删多余键：需要和 jsonschema 一致的正则方言（ECMA 262）。**没做**；现在遇到 patternProperties 时一律不删，所以不会错删。
4. 浮点数 → 字符串、`"yes"`/`"1"` → true：语义不唯一（`1.50` 原来的写法已经丢了；`"1"` 是不是布尔也说不准）。**按设计不做。**
5. 流式增量修复：spec 第 2 节明确本期不做。

【推断】
- 语料类别是我设计的，其中 enum_case、null_as_string 这类正好是 schemafix 实现了的纠正。json_repair 独有的能力（yes/no 转布尔、浮点转字符串、patternProperties、salvage 的列表转对象）大多没进语料。"全对"不能理解为对任意真实 LLM 输出都全对；见 `docs/benchmark.md` 的偏差说明。

## 附：手工核实命令

1. json_repair 的行为（在 `bench/python` 下执行）：
   ```bash
   uv run python -c "import json_repair; s={'type':'object','properties':{'n':{'type':'integer'},'s':{'type':'string'},'b':{'type':'boolean'}}}; [print(r, json_repair.loads(r, schema=s, logging=True)) for r in ['{\"n\":\"3.0\"}','{\"s\":1.5}','{\"b\":\"yes\"}','{\"n\":\"1e30\"}']]"
   ```
   本机输出：`3.0`→3；`1.5`→`"1.5"`；`"yes"`→True；`"1e30"`→1000000000000000019884624838656。另外 `{"n":[3]}` 会抛出 `Expected integer at $.n.`。
2. ajv `coerceTypes:"array"` 的行为（在 `bench/node` 下执行）：
   ```bash
   node -e "const A=require('ajv/dist/2020.js').default;const a=new A({coerceTypes:'array',strict:false});const v=a.compile({type:'object',properties:{n:{type:'integer'},s:{type:'string'},b:{type:'boolean'},z:{type:'null'}}});for(const d of [{n:'3.0'},{n:[3]},{s:1.5},{b:'yes'},{z:''},{z:'null'}]){console.log(JSON.stringify(d),v(d))}"
   ```
   本机输出：`{"n":3} true`、`{"n":3} true`、`{"s":"1.5"} true`、`{"b":"yes"} false`、`{"z":null} true`、`{"z":"null"} false`。
