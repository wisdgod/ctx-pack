# 清单文件格式 (.ctx.manifest)

## 核心定位

清单文件是输出文件的**可操作索引**。

它精确记录每个块在输出文件中的位置(字节偏移+行范围)，
使得增量更新时可以直接seek到对应位置局部重写，
而非每次重新生成整个输出文件。

对大代码库（输出文件数百KB甚至MB级），这是核心性能设计。

## 格式

```yaml
version: 1
prefix: "ctx"
profile: "default"
generated_at: "2025-01-15T10:30:00Z"

output_file: "context.ctx"
output_size_bytes: 48230
file_count: 12

blocks:
  - type: prompt
    byte_start: 0
    byte_end: 1240
    line_start: 1
    line_end: 15

  - type: tree
    byte_start: 1241
    byte_end: 1890
    line_start: 16
    line_end: 30

  - type: file
    fid: 1
    gen: 0
    path: "src/main.rs"
    byte_start: 1891
    byte_end: 4320
    line_start: 32
    line_end: 89
    content_hash: "sha256:abcdef1234..."

  - type: patch
    fid: 1
    gen: 0
    pid: 1
    byte_start: 4321
    byte_end: 4580
    line_start: 91
    line_end: 102

  - type: file
    fid: 2
    gen: 0
    path: "src/lib.rs"
    byte_start: 4581
    byte_end: 8900
    line_start: 104
    line_end: 220
    content_hash: "sha256:789abc..."

tag_occurrences:
  "ctx:prompt": [[1, 15]]
  "ctx:tree": [[16, 30]]
  "ctx:file": [[32, 89], [104, 220]]
  "ctx:patch": [[91, 102]]
```

## 增量重写流程

1. 读取旧manifest
2. 确定哪些fid的内容变化了（index hash对比）
3. 对未变化的块：保持输出文件中该区间不动
4. 对变化的块：在对应byte位置重写
5. 如果新块比旧块大/小：调整后续块偏移
6. 更新manifest中所有偏移

注意：如果变化的块很多或偏移调整级联严重，
退化为全量重写（仍然正确，只是没有性能优势）。
