# Patch/Replace块格式

## Patch块

```
<{prefix}:patch fid="{fid}" gen="{gen}" pid="{pid}">
@@ anchor:{line_num} @@
 [4]context line (unchanged)
-[4]removed line
+[4]added line
+[4]another added line
@@ anchor:{line_num} @@
-[8]old code
+[8]new code
</{prefix}:patch>
```

### 语法规则

- `@@ anchor:N @@` 定位hunk，N是最近的锚定行号
- ` ` (空格前缀) 上下文行，不变
- `-` 删除行
- `+` 添加行
- 无前缀行视为上下文行
- 缩进编码(`[N]`)保持一致
- 多个hunk用多个 `@@ @@` 分隔

### Anchor定位语义

anchor:N 表示"在锚定行号N附近"。
Apply时先找到精确的anchor行，然后用上下文行微调定位。
如果文件被修改导致anchor偏移，fuzzy match用上下文行内容搜索。

### Fuzzy Match策略

1. 先尝试精确anchor定位
2. 失败则在anchor±window范围内搜索上下文行序列
3. window大小可配(默认20行)
4. 全部失败 → .rej文件

## Replace块

```
<{prefix}:replace fid="{fid}" gen="{gen}">
[编码后全量内容，格式与file块内部完全相同]
</{prefix}:replace>
```

Replace表示完全替换文件内容。gen值已递增。
旧gen的所有file/patch块作废。
