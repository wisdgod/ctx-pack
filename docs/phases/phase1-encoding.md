# Phase 1: 编码层

## 目标
实现双向TransformStage trait和两个编码器。
纯函数，零IO，重测试。

## 依赖
Phase 0 (项目结构存在)

## 产出文件

### src/encoding_layer/traits.rs

```rust
/// 编码管道中的一个变换阶段。
/// 不变量: 对任意合法输入, decode(encode(input)) == input
pub trait TransformStage {
    /// 将原始文本转为编码文本
    fn encode(&self, input: &str) -> String;
    /// 将编码文本还原为原始文本
    fn decode(&self, input: &str) -> String;
}
```

### src/encoding_layer/indent.rs

`IndentEncoder` 实现 `TransformStage`。

构造: `IndentEncoder::new(tab_width: u32)`

Encode逐行处理:
- 空行(len=0 或只有\n) → 保持空行
- 计算leading spaces（tab按tab_width换算）
- 输出 `[{N}]{rest_of_line}`
- 行首非空白且非tab → `[0]{line}`

Decode逐行处理:
- 不以 `[` 开头 → 原样返回（空行等）
- 匹配 `\[(\d+)\](.*)` → N个空格 + 捕获组2

边界case:
- `[0]` 单独一行（原始为空白行？不，空白行encode为空） → decode为空字符串
  实际上 `[0]` 不应该单独出现，`[0]text` decode为 `text`
- `[4]` 无后续内容 → decode为 `    `（4个空格的纯空白行）

### src/encoding_layer/anchor.rs

`AnchorEncoder` 实现 `TransformStage`。

构造: `AnchorEncoder::new(interval: u32)`

interval=0时encode/decode均为identity（直接返回输入）。

Encode:
- 将输入按行split
- 计算总行数 → 确定行号对齐宽度 (总行数的十进制位数 + 1，最小4)
- 第1行始终标注行号
- 此后每interval行标注
- 标注格式: `{num:>width} | {content}`
- 非标注行: `{spaces:>width} | {content}` (spaces = width个空格)

Decode:
- 逐行匹配模式 `^\s*\d*\s*\| (.*)$`
- 提取 `| ` 之后的内容
- 如果行不匹配模式 → 原样返回（容错）

### src/encoding_layer/pipeline.rs

`Pipeline` struct:
- `stages: Vec<Box<dyn TransformStage>>`

方法:
- `Pipeline::new() -> Self`
- `Pipeline::add_stage(stage: impl TransformStage + 'static)`
- `Pipeline::encode_all(&self, input: &str) -> String`
  - stages按顺序执行encode
- `Pipeline::decode_all(&self, input: &str) -> String`
  - stages按**反序**执行decode

### src/encoding_layer/mod.rs

公开: `TransformStage`, `IndentEncoder`, `AnchorEncoder`, `Pipeline`

提供工厂函数:
- `build_pipeline(config: &GlobalConfig) -> Pipeline`
  - 根据配置决定添加哪些stage

## 测试要求

### 单元测试 (indent.rs)

```
encode("") == ""
encode("hello") == "[0]hello"
encode("    let x = 1;") == "[4]let x = 1;"
encode("        deep();") == "[8]deep();"
encode("\t\tcode") == "[8]code"  (tab_width=4)
encode("    ") == "[4]"  (纯空白行)
decode("[4]let x = 1;") == "    let x = 1;"
decode("[0]hello") == "hello"
decode("") == ""
decode("[4]") == "    "

// round-trip
for text in [多种测试输入]:
    assert_eq!(decode(encode(text)), text)
```

### 单元测试 (anchor.rs)

```
// interval=5, 12行输入
encode后:
  第1行有行号 "   1 | ..."
  第2-4行无行号 "     | ..."
  第5行有行号 "   5 | ..."
  第10行有行号 "  10 | ..."

// interval=0
encode(text) == text
decode(text) == text

// round-trip
decode(encode(text)) == text
```

### 集成测试 (pipeline.rs)

```
// 构建pipeline: indent + anchor
let pipeline = Pipeline::new();
pipeline.add_stage(IndentEncoder::new(4));
pipeline.add_stage(AnchorEncoder::new(10));

let original = "fn main() {\n    let x = 1;\n}\n";
let encoded = pipeline.encode_all(original);
let decoded = pipeline.decode_all(&encoded);
assert_eq!(decoded, original);

// 大文件round-trip (100+行)
```

## 注意事项

- encode和decode处理的是完整的多行文本(&str)，内部按行处理
- 行尾换行符的处理要一致：如果输入末尾有\n，输出末尾也有\n
- anchor的行号基于encode输入的行数，不是原始文件行数
  （在pipeline中，anchor的输入是indent encode后的文本，行数相同）
