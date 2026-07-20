# 输出文件格式 (.ctx)

## 整体结构

```
<{prefix}:prompt>
[自动生成的协议说明]
</{prefix}:prompt>

<{prefix}:tree>
[文件树索引]
</{prefix}:tree>

<{prefix}:file id="1" gen="0" path="src/main.rs">
[编码后内容]
</{prefix}:file>

<{prefix}:file id="2" gen="0" path="src/lib.rs">
[编码后内容]
</{prefix}:file>

... 更多file块 ...

... patch/replace块(增量模式时) ...
```

## prompt块

动态生成，内容取决于当前配置启用了哪些编码功能。
包含:
- 标签含义说明
- 编码规则解释(如果indent_encoding=true)
- 锚定行号解释(如果anchor_interval>0)
- 版本寻址模型说明
- LLM输出patch的格式指引

这是使整个协议自解释的关键。没有它，LLM无法正确理解编码后内容，
也无法输出格式正确的patch。

## tree块

```
<ctx:tree>
[1] src/main.rs (gen0)
[2] src/lib.rs (gen0)
[3] src/utils/helpers.rs (gen1.pid2)
</ctx:tree>
```

格式: `[{fid}] {relative_path} ({version_summary})`
版本摘要: `gen{N}` 表示全量, `gen{N}.pid{M}` 表示有补丁。

## file块

```
<ctx:file id="{fid}" gen="{gen}" path="{relative_path}">
   1 | [0]fn main() {
     | [4]let x = 1;
     ...
  10 | [0]}
</ctx:file>
```

当前实现的属性: `id`, `gen`, `path`。
`id` 与 patch/replace 中的 `fid` 是同一个文件编号。
当文件来自partial extraction时，会额外输出 `extraction="partial"`。

## 部分提取输出

当extraction使用lines或regex模式时，当前实现会把选中的fragment内容按顺序拼接后编码，
不会输出省略标记或fragment边界。

这类输出目前是**阅读视图**。因为没有fragment边界和原始行号映射，
LLM基于partial视图生成的patch/replace不能可靠应用回完整源文件。
需要可反向apply的文件应使用full extraction。

启用anchor行号时，partial输出中的行号是拼接后的提取视图行号，
不是原始源文件行号。

partial file块示例:

```
<ctx:file id="5" gen="0" path="src/large.rs" extraction="partial">
   1 | [0]pub fn important() {
     | [4]todo!()
</ctx:file>
```

早期设计曾考虑如下省略标记，当前未实现:

```
<ctx:file id="5" gen="0" path="src/large.rs">
   1 | [0]// file header
     | [0]use std::io;
     |
<ctx:omit lines="4-49"/>
     |
  50 | [0]pub fn important() {
     | [4]todo!()
  60 | [0]}
</ctx:file>
```
