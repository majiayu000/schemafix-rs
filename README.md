# schemafix-rs

把 LLM 的原始结构化输出按 JSON Schema **修复 → 纠正 → 校验**，返回可直接使用的值，或一段能回喂给模型重试的错误提示。

```
原始文本 → jsonrepair-rs（语法修复）→ schema 纠正（每条记为 Fix）→ jsonschema 校验 → Outcome
```

状态：v0.1，尚未发布到 crates.io。

## 库用法

```toml
[dependencies]
schemafix-rs = { path = "../schemafix-rs" }            # lib 名为 `schemafix`
# schemafix-rs = { path = "...", features = ["schemars"] }  # 需要 parse::<T>
```

```rust
use schemafix::{Fixer, FixOptions, Outcome};
use serde_json::json;

let schema = json!({
    "type": "object",
    "properties": {
        "days": { "type": "integer" },
        "unit": { "enum": ["celsius", "fahrenheit"] }
    },
    "required": ["days", "unit"]
});
let fixer = Fixer::new(&schema)?; // 编译一次，可复用

match fixer.fix(r#"{days: "3", unit: "Celsius"}"#) {
    Outcome::Valid { value } => { /* 原样通过 */ }
    Outcome::Fixed { value, fixes } => {
        // value == {"days": 3, "unit": "celsius"}
        for fix in &fixes { println!("{fix}"); } // 每条都有 path / kind / before / after
    }
    invalid @ Outcome::Invalid { .. } => {
        // 把这段文本放进下一轮 prompt，由调用方决定是否重试
        let hint = invalid.retry_hint().unwrap();
    }
}
```

`FixOptions`：

| 字段 | 默认 | 作用 |
|---|---|---|
| `fill_defaults` | `true` | `required` 缺失且该属性 schema 有 `default` 时补上 |
| `drop_additional_properties` | `false` | `additionalProperties: false` 时删除多余键 |
| `max_input_bytes` | `None` | 超长输入直接 `Invalid`，不解析 |

带类型（feature `schemars`，schema 由 `schemars` 生成）：

```rust
let (weather, fixes) = schemafix::parse::<Weather>(raw)?;
// 失败时 Err(FixError::Invalid(outcome))，outcome.retry_hint() 可用
```

## CLI

```bash
cargo run --bin schemafix -- --schema schema.json < llm_output.txt
```

可选参数：`--drop-additional`、`--no-defaults`、`--max-input-bytes <n>`。
stdout 输出最终 JSON（无法解析时不输出），stderr 输出每条 Fix 及失败时的 retry hint。

| 退出码 | 含义 |
|---|---|
| 0 | 原样通过 |
| 1 | 修复后通过 |
| 2 | 仍不符合 schema（或文本无法解析为 JSON） |
| 3 | schema 文件读不到 / 不是 JSON / 编译失败、stdin 读失败、参数错误 |

## 会做的纠正

| FixKind | 触发条件 | 不会做的情况 |
|---|---|---|
| `syntax_repair` | 文本不是合法 JSON，jsonrepair-rs 能修 | — |
| `string_to_number` | `type` 含 `integer`/`number` 且不含 `string` | 小数给 `integer`；转换会丢精度（如 `"12345678901234567890123"`） |
| `string_to_boolean` | `type` 含 `boolean`，值为 `"true"`/`"false"`（ASCII 不区分大小写） | `"yes"`、`"1"` 等 |
| `string_to_null` | `type` 含 `null` 且不含 `string`，值恰为 `"null"` | schema 允许字符串时 |
| `enum_case` | 字符串忽略 ASCII 大小写后**唯一**匹配一个 enum 成员 | 匹配多个（如 `"Auto"`/`"AUTO"`） |
| `wrap_in_array` | `type` 含 `array`，值是非 null 的单值且不属于其它允许的类型 | `null`；值已是允许的其它类型 |
| `fill_default` | 见 `fill_defaults` | 非 required 属性；没有 `default` |
| `drop_additional_property` | 见 `drop_additional_properties` | 有 `patternProperties` 时（不做正则判断） |

递归：`properties`、`items`（单 schema 形式）、本地 `$ref`（`#`、`#/$defs/...`、`#/definitions/...`）。
`anyOf`/`oneOf`：值已满足某个分支就不动；否则按顺序取第一个"纠正后能通过该分支校验"的分支；`oneOf` 还要求纠正后的值只匹配一个分支。

安全原则：输入一开始就通过校验时不做任何 schema 纠正；有歧义时保持原值，交给校验进入 `Invalid`。

## 限制

- 不调用 LLM、不做重试循环；只给 `retry_hint()`。
- 不支持远程 `$ref`（`jsonschema` 关闭了默认的 http/file 解析，含远程引用的 schema 在 `Fixer::new` 时报错）。
- `allOf`、`if/then/else`、`prefixItems`、元组形式 `items`、`patternProperties`、`dependentSchemas` 等关键字只参与校验，不参与纠正。
- `anyOf`/`oneOf` 分支单独编译时只携带根上的 `$schema`/`$defs`/`definitions`；分支里指向根其它位置的引用（如 `#/properties/x` 或 `#`）无法解析，该分支会被跳过。
- `$ref` 的 JSON Pointer 不做百分号解码。
- 文本前后夹带说明文字（如先写一句 "Sure!" 再跟代码块）能否修好取决于 jsonrepair-rs；当前版本会失败并进入 `Invalid`。
- 每次 `fix` 中遇到 `anyOf`/`oneOf` 都会重新编译分支，没有缓存。
- 真实 OpenAI / Anthropic tool calling 坏输出的修复通过率：**尚无真实样本数据，未测**。

## 开发

```bash
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
```
