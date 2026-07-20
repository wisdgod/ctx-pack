# Phase 3: 内容提取

## 目标
从UTF-8文本中按规则提取片段。

## 新增依赖

```toml
regex = "1"
```

## 产出文件

### src/extraction.rs

```rust
pub struct Fragment {
    pub line_start: u32,  // 原始文件中的起始行号(1-based)
    pub line_end: u32,    // 原始文件中的结束行号(inclusive)
    pub content: String,  // 提取的文本内容
}

pub struct ExtractionResult {
    pub fragments: Vec<Fragment>,
    pub total_lines: u32,  // 原始文件总行数
    pub is_partial: bool,  // 是否为部分提取
}
```

函数: `extract(content: &str, rule: &ExtractionRule) -> ExtractionResult`

根据rule.mode分发。

### src/extraction/full.rs

函数: `extract_full(content: &str) -> ExtractionResult`

单个Fragment包含全部内容。is_partial = false。

### src/extraction/lines.rs

函数: `extract_lines(content: &str, ranges: &str) -> Result<ExtractionResult>`

解析ranges字符串: "1-20,50-60,100-"
- `N-M`: 第N到第M行(inclusive)
- `N-`: 第N行到文件末尾
- `N`: 单行

对每个范围生成一个Fragment。is_partial = true(除非range覆盖全文)。

### src/extraction/regex_extract.rs

函数: `extract_regex(content: &str, pattern: &str, context_lines: u32) -> Result<ExtractionResult>`

1. 编译regex
2. 逐行匹配
3. 匹配行 ± context_lines 形成一个区域
4. 合并重叠区域
5. 每个区域生成一个Fragment
6. is_partial = true

### 规则匹配器

在 `src/extraction.rs` 中:

函数: `match_rule<'a>(path: &str, rules: &'a [ExtractionRule], default_mode: ExtractionMode) -> &'a ExtractionRule`

按顺序匹配rules中的glob pattern。
第一个match的规则生效。
无匹配时返回default_mode对应的默认规则。

## 测试要求

- lines: "1-5" 从10行文本中取前5行
- lines: "3-5,8-10" 产出2个fragment
- lines: "5-" 取第5行到末尾
- regex: 简单pattern匹配，context_lines=0
- regex: context_lines=2，验证重叠合并
- 规则匹配: first-match-wins验证
