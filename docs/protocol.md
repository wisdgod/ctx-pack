# 表征协议规范

ctx-pack的输出遵循一套可扩展的表征协议。
协议由编码层、版本模型、标签体系三部分组成。

## 编码层

编码层是一个有序的transform管道。每个stage实现双向转换(encode/decode)。
管道设计使未来引入任意压缩算法（RLE等）只需添加新stage。

### 不变量

对任意stage和任意输入text:
```
decode(encode(text)) == text
```

对整个pipeline:
```
pipeline.decode_all(pipeline.encode_all(text)) == text
```

例外: 当前 `IndentEncoder` 会把行首 tab 按 `tab_width` 归一化为空格。
因此包含行首 tab 的输入满足的是“解码到归一化后的文本”，不是逐字节还原。
如果需要保留 tab/space 的原始混合形态，应关闭 `indent_encoding` 或后续引入保真缩进编码。

### Stage 1: 绝对缩进编码 (IndentEncoder)

输入: raw文本行
输出: `[N]` 前缀行

规则:
- 计算行首连续空格数 N
- 替换为 `[N]` 前缀 + 剩余内容
- 空行(仅换行) → 空行(不加前缀)
- 纯空白行 → `[N]`(N为空白数，无后续内容)
- Tab: 1 tab = 配置中的tab_width(默认4)个空格；这是有损归一化，decode 会恢复为空格

示例:
```
输入: "        let x = 1;"
输出: "[8]let x = 1;"

输入: ""
输出: ""

输入: "    "
输出: "[4]"

输入: "no indent"
输出: "[0]no indent"
```

逆向:
```
输入: "[8]let x = 1;"
输出: "        let x = 1;"
```

### Stage 2: 锚定行号 (AnchorEncoder)

输入: 文本行序列(已经过Stage 1)
输出: 带行号前缀的行

参数: `anchor_interval` (如10)

规则:
- full extraction 时行号基于原始文件行号(1-based)
- lines/regex partial extraction 当前基于“拼接后的提取视图”重新编号；该输出只适合阅读，不保证可反向 apply
- 行号对齐宽度 = 当前编码视图总行数的位数 + 1
- 每 anchor_interval 行标注
- 第1行始终标注
- 非锚定行用空格填充对齐
- 分隔符: ` | `

示例 (interval=10, 总行数=120 → 宽度4):
```
   1 | [0]fn main() {
     | [4]let x = 1;
     | [4]let y = 2;
     ...
  10 | [4]return x + y;
  11 | [0]}
     |
     | [0]fn helper() {
     ...
  20 | [4]todo!()
```

逆向:
- 匹配行首 `\s*\d*\s*\| ` 模式并剥离
- 恢复Stage 1编码后的文本

### Stage N (future): 通用重复压缩

预留接口。任何实现 TransformStage trait 的struct均可插入pipeline。

## 版本模型

### 寻址

```
(fid, gen, pid)

fid: u32  — 文件编号
gen: u32  — 基线代号(generation)
pid: u32  — 补丁序号
```

### 状态转换

```
初始:         (fid, gen=0, pid=0) → 全量内容 <prefix:file>
小变化patch:  (fid, gen,   pid+1) → 差异块   <prefix:patch>
大变化replace:(fid, gen+1, pid=0) → 全量内容 <prefix:replace>
```

### 裁剪语义

replace发出后，同fid的所有旧gen块作废。
在增量对话中，LLM应忽略旧gen的file/patch块。

### 工具内部简化

不重播patch链。
每次pack后缓存当前raw内容为最新快照。
下次diff = encode(上次快照, 当前配置) vs encode(当前文件, 当前配置)。

## 标签体系

### 前缀

所有标签使用可配置前缀，格式: `<{prefix}:{tag}>`。
默认前缀 `ctx`，可在配置中修改，冲突时用 `migrate-prefix` 命令迁移。

不做XML转义。如果文件内容碰巧包含相同标签，用户应迁移前缀。

### 标签清单

| 标签 | 用途 | 属性 |
|------|------|------|
| `prefix:prompt` | 协议自描述文本 | 无 |
| `prefix:tree` | 文件树索引 | 无 |
| `prefix:file` | 文件内容 | id, gen, path, extraction(可选, partial时输出；`id` 即 fid) |
| `prefix:patch` | 增量补丁 | fid, gen, pid |
| `prefix:replace` | 全量替换 | fid, gen |

### Patch内容投影

`file` 和 `replace` 块内部使用完整编码输出，包括 anchor 行号栏。
`patch` hunk 行使用去掉左侧行号栏后的内容投影:

```
   1 | [0]fn main() {
     | [4]old();
```

在 patch 中写作:

```
@@ anchor:1 @@
 [0]fn main() {
-[4]old();
+[4]new();
```

`@@ anchor:N @@` 中的 N 仍引用 file/replace 块左侧可见的 anchor 行号。
这样 LLM 不需要复制空白对齐和 `|` 分隔符，只需要保持内容行本身一致。

### base_indent属性

早期设计保留过 `base_indent="N"` 属性，用于描述片段在原始文件中的典型起始缩进。
当前实现不输出该属性；缩进编码始终使用绝对 `[N]` 值。
