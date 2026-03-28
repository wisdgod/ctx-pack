# Phase 8: 收尾功能

## 目标
完成所有辅助命令和边缘功能。

## 产出

### migrate-prefix命令

`src/migrate/prefix.rs`

函数: `migrate_prefix(old: &str, new: &str, config: &Config) -> Result<()>`

1. 读取所有输出文件(.ctx)
2. 文本替换: `<{old}:` → `<{new}:`, `</{old}:` → `</{new}:`
3. 更新manifest中的tag_occurrences
4. 更新配置文件中的prefix字段
5. 报告替换数量

### cache命令

`cache clean --profile NAME`:
- 加载index
- 对所有文件执行cleanup(retain = config.cache_retention)

`cache info`:
- 统计cache目录大小
- 列出快照数量

### init命令

生成带注释的默认 `ctx-pack.yaml`。
注释解释每个字段的含义和可选值。

### tree命令

`ctx-pack tree --profile NAME`:
- Discovery + Index(如果存在)
- 输出tree（与输出文件中的tree块格式相同）
- 如果index存在，显示版本信息

### prompt命令

`ctx-pack prompt --profile NAME`:
- 加载config
- 生成prompt文本（与嵌入输出文件中的相同）
- 输出到stdout
- 用途: 用户预览/调试prompt内容

### 大小警告集成

在 pack::output 中:
- 每处理一个文件后累加大小
- 单文件超阈值 → 按size_policy处理
- 总量超阈值 → 按size_policy处理
- warn: tracing::warn!
- abort: 返回错误
- ignore: 不检查

## 验证标准

- migrate-prefix: 替换后输出文件中无旧前缀
- cache clean: 旧快照被删除
- init: 生成的配置文件可被load
- tree: 输出格式正确
- prompt: 输出与pack中嵌入的一致
- 大小警告: 超阈值时warn/abort行为正确
