# ctx-pack

## 定位

配置驱动的源文件归一化工具。将代码库内容打包为LLM可读的结构化上下文文件，
内建版本协议支持增量patch/replace，支持反向应用LLM生成的修改。

## 核心理念

1. **表征协议，非格式**：输出不是简单的文件拼接，而是一套可扩展的表征协议，
   包含编码层、版本寻址、自描述prompt。协议的每一层都为token效率和LLM认知优化。

2. **配置即真相**：所有行为由YAML配置决定。不确定的行为交给配置项，CLI参数只做覆盖。

3. **迁就AI的容错设计**：我们简化patch格式（anchor-based而非精确行号），
   因为apply端有fuzzy match容错。工具适应AI的不精确性，而非要求AI精确。

4. **增量一切**：文件发现是增量的（索引持久化），内容输出是增量的（patch/replace），
   输出文件本身的更新也是增量的（manifest驱动局部重写）。

## 术语表

| 术语 | 含义 |
|------|------|
| fid | 文件编号，持久分配给路径，永不回收 |
| gen | 基线代号(generation)，replace时递增 |
| pid | 补丁序号，patch时递增 |
| anchor | 锚定行号，每N行标注一次的原始文件行号 |
| prefix | XML标签前缀，如 `ctx`，产出 `<ctx:file>` 等标签 |
| base_indent | 片段的典型起始缩进级别（元信息，不参与编码计算） |
| snapshot | 文件raw内容的缓存副本，存于 .ctx-cache |
| manifest | 描述输出文件结构的清单文件，支持局部重写 |
| profile | 配置中的命名规则集，对应一种打包策略 |

## 技术栈

- Rust 2021 edition
- clap (derive API) — CLI
- serde + serde_yaml — 配置
- ignore + globset — 文件发现
- content_inspector — 二进制检测
- encoding_rs — 字符编码检测与转换
- similar — 行级diff
- sha2 — 内容hash
- regex — 正则提取
- anyhow + thiserror — 错误处理
- tracing + tracing-subscriber — 日志
