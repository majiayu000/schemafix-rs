# Spec：schemafix-rs（暂定名）—— LLM 结构化输出的校验 + 修复 + 纠正

> 状态：草案，待确认。仓库名、crate 名都未在 crates.io 查重。

## 1. 为什么做

【事实】
- jsonrepair-rs 只保证"语法合法的 JSON"；`jsonrepair_value_with_schema`（`src/lib.rs` 约 366 行起）只做 best-effort 纠正：
  数字/布尔字符串转换、大小写不敏感 enum、单值包数组、嵌套 `properties` / `items`。它的文档明确写着"不是 JSON Schema 校验"。
- 底层能力 Rust 已有：`jsonschema`（校验，1.31 亿下载）、`llguidance`（约束解码，223 万下载）。
- 上层"校验失败 → 定位 → 按 schema 修 → 再校验 → 给出可回传给 LLM 的错误"在 Rust 里只有 rstructor（34★）。

【推断】调用方真正想要的是一个函数：**给我原始 LLM 文本 + schema，返回符合 schema 的值，或者一份能直接回喂给模型重试的错误报告**。

## 2. 范围

### 做
1. 管线：`原始文本 → jsonrepair（语法）→ schema 纠正 → jsonschema 校验 → 结果/报告`
2. schema 纠正（从 jsonrepair-rs 迁入并扩展）：
   - 已有：number/integer/boolean 字符串转换、enum 大小写、单值包数组、properties/items 递归
   - 新增：`required` 缺失但有 `default` 时补默认值；`additionalProperties: false` 时删多余键（可配置）；
     `oneOf/anyOf` 选第一个能通过校验的分支；`$ref`（本地 `#/$defs`）解析；字符串 `"null"` → null（仅当 schema 允许 null）
3. 每一次纠正都记录为 `Fix { path: JsonPointer, kind, before, after }`，调用方能审计
4. 校验失败时生成 `RetryHint`：按 JSON Pointer 列出错误，渲染成一段可直接放进下一轮 prompt 的文本
5. CLI：`schemafix --schema s.json < llm_output.txt`，退出码区分 通过 / 已修复 / 失败
6. `serde` 路径：`schemafix::parse::<T>(text)`，schema 来自 `schemars`（feature `schemars`）

### 不做（本期）
- 不调用任何 LLM、不做重试循环（只给 `RetryHint`，循环由调用方写）
- 不做约束解码 / 不导出 GBNF（列为 v0.2 候选，届时依赖 llguidance，不自己实现）
- 不做流式增量校验
- 不支持远程 `$ref`

## 3. 公开接口（草案）

```rust
pub struct Fixer { /* 编译后的 schema + 选项 */ }

impl Fixer {
    pub fn new(schema: &serde_json::Value) -> Result<Self, SchemaError>;
    pub fn with_options(schema: &serde_json::Value, opts: FixOptions) -> Result<Self, SchemaError>;
    pub fn fix(&self, raw: &str) -> Outcome;
}

pub struct FixOptions {
    pub drop_additional_properties: bool, // 默认 false
    pub fill_defaults: bool,              // 默认 true
    pub max_input_bytes: Option<usize>,
}

pub enum Outcome {
    Valid { value: Value },                          // 原样通过
    Fixed { value: Value, fixes: Vec<Fix> },         // 修复后通过
    Invalid { value: Option<Value>, fixes: Vec<Fix>, errors: Vec<SchemaViolation> },
}

impl Outcome {
    pub fn retry_hint(&self) -> Option<String>;      // 仅 Invalid 返回
}

#[cfg(feature = "schemars")]
pub fn parse<T: DeserializeOwned + JsonSchema>(raw: &str) -> Result<(T, Vec<Fix>), FixError>;
```

## 4. 依赖

| crate | 用途 | 备注 |
|---|---|---|
| jsonrepair-rs | 语法修复 | 自有 |
| jsonschema | 校验 | 默认关闭其网络 `$ref` 解析 |
| serde_json | 值模型 | |
| schemars | 由类型生成 schema | 可选 feature |

## 5. 对现有仓库的影响（需要你确认）

- **jsonrepair-rs**：按"不做向后兼容"原则，`jsonrepair_value_with_schema` 及 `correct_value_with_schema`、`matching_enum_string`、
  `numeric_string_round_trips`、`normalized_decimal` 迁到新 crate 后从 jsonrepair-rs 删除，发 0.3.0（破坏性变更）。
  它的下游（wecom-cli、OpenBitFun、OpenFlow）是否调用了这个函数，删除前需要先查。
- 若你不想动 jsonrepair-rs，则新 crate 先复制这段逻辑——这会违反"同类逻辑只保留一个来源"，需要你明确选择。

## 6. 目录结构（每个文件 < 200 行）

```
src/lib.rs          公开 API 与 Outcome
src/fixer.rs        管线编排
src/correct/        scalar.rs  enum_.rs  array.rs  object.rs  combinators.rs  refs.rs
src/fix.rs          Fix 记录与 JsonPointer
src/hint.rs         RetryHint 渲染
src/bin/schemafix.rs
tests/              每种纠正一组正/反例；真实 LLM 坏输出样本集 fixtures/
benches/
```

## 7. 验收标准

1. `cargo test` 全绿；每一种纠正都有"应修"和"不应修"两类测试
2. 不安全纠正绝不发生：语义有歧义（如 enum 多个大小写匹配、数字精度丢失）时保持原值并进入 `Invalid`
3. 对 OpenAI / Anthropic tool calling 的公开 schema 示例，各收集 ≥ 30 条真实坏输出，报告"修复后通过率"，结果写进 README（无数据就不写）
4. CLI 退出码：0 通过、1 已修复、2 失败、3 输入/schema 错误
5. 与 rstructor 的对比只写可复现的命令与结果

## 8. 待你决定

- crate / 仓库名（schemafix-rs 只是占位）
- 是否从 jsonrepair-rs 删除 schema 纠正（第 5 节）
- 仓库建在哪里、是否公开
