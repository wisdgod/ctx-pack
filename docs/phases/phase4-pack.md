# Phase 4: 基础Pack（端到端可用）

## 目标
`ctx-pack pack --full` 可以产出完整的 .ctx 文件和 manifest。
不含版本系统（全部输出为file块，gen=0, pid=0）。

## 产出文件

### src/pack/tree.rs

函数: `generate_tree(files: &[(u32, &str)]) -> String`
- 输入: (fid, display_path) 列表
- 输出: tree文本块

格式:
```
[1] src/main.rs (gen0)
[2] src/lib.rs (gen0)
```

此阶段全部gen0，后续Phase 6扩展版本信息。

### src/pack/prompt.rs

函数: `generate_prompt(config: &GlobalConfig) -> String`

根据配置动态生成协议说明:
- 如果indent_encoding=true → 解释[N]语法
- 如果anchor_interval>0 → 解释锚定行号
- 解释标签含义
- 说明LLM应如何输出patch

这不是硬编码模板。是条件拼装的文本。
每种编码功能启用时才包含对应说明。

### src/pack/output.rs

函数: `pack_full(config: &Config, profile_name: &str) -> Result<PackOutput>`

```rust
pub struct PackOutput {
    pub content: String,        // 完整输出文本
    pub blocks: Vec<BlockInfo>, // 每个块的位置信息(给manifest用)
}

pub struct BlockInfo {
    pub block_type: BlockType,
    pub fid: Option<u32>,
    pub gen: u32,
    pub pid: Option<u32>,
    pub path: Option<String>,
    pub byte_start: u64,
    pub byte_end: u64,
    pub line_start: u32,
    pub line_end: u32,
    pub content_hash: Option<String>,
}
```

完整流程:
1. 加载配置 + 获取profile
2. Discovery → 文件列表
3. 对每个文件: Detection → Extraction → Encode Pipeline
4. 大小检查（单文件 + 总量）
5. 组装: prompt + tree + file blocks
6. 记录每个块的位置信息
7. 返回PackOutput

### src/pack/manifest.rs

函数: `write_manifest(output: &PackOutput, config: &Config) -> Result<()>`

将blocks信息序列化为manifest YAML。

### src/pack/mod.rs

整合上述，提供 `pack` 命令的完整handler。

### CLI集成

`src/main.rs` 中 pack 子命令调用 `pack::pack_full`。
将content写入output file。
将manifest写入manifest file。

## 验证标准

- 准备一个含3-5个Rust文件的测试项目
- `ctx-pack pack --full -o test.ctx` 生成输出
- 输出包含 prompt + tree + file blocks
- manifest记录正确的行偏移
- 编码层正确应用（缩进编码 + 锚定行号）
- 大文件触发size warning

## 测试要求

- tree生成格式
- prompt生成：开/关各种编码功能，验证输出变化
- pack集成：小项目端到端
