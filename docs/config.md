# 配置规范

配置文件: `ctx-pack.yaml`，位于项目根目录。

## 完整Schema

```yaml
global:
  prefix: "ctx"                  # string, 标签前缀
  anchor_interval: 10            # u32, 锚定行号间隔, 0=禁用
  indent_encoding: true          # bool, 缩进编码开关
  tab_width: 4                   # u32, 行首tab转空格数
  binary_policy: skip            # enum: skip|warn|abort
  encoding_detection: true       # bool, 非UTF-8自动转换
  max_content_size: "500KB"      # size string, 总输出警告阈值
  max_file_size: "100KB"         # size string, 单文件警告阈值
  size_policy: warn              # enum: warn|abort|ignore
  index_file: ".ctx-index.yaml"  # path
  cache_dir: ".ctx-cache"        # path
  cache_retention: 5             # u32, 保留最近N个gen的快照
  manifest: true                 # bool
  prompt_generation: true        # bool

profiles:
  <name>:                        # string, profile名称
    roots:                       # list
      - path: "."               # string, 根路径
        label: "project"        # string, tree中显示的标签
    discovery:
      use_gitignore: true        # bool
      include: []                # list of glob patterns
      exclude: []                # list of glob patterns
      stdin_merge: false         # bool, 合并stdin文件列表
    extraction:
      default_mode: full         # enum: full|lines|regex
      rules:                     # list, first-match-wins
        - match: "glob"         # glob pattern匹配路径
          mode: full|lines|regex
          # lines模式:
          ranges: "1-20,50-60"  # 行范围表达式
          # regex模式:
          pattern: "regex"      # 正则表达式
          context_lines: 2      # u32, 匹配行上下文
    versioning:
      auto_diff: true            # bool
      replace_threshold: 0.5     # f64, 0.0-1.0
      max_patches_before_replace: 5  # u32
    output:
      file: "context.ctx"       # path
      manifest: "context.ctx.manifest"  # path
```

## 字段说明

### size string格式
支持: "500KB", "10MB", "1GB", 纯数字表示字节。
解析时不区分大小写。

### 行范围表达式
格式: `"start-end,start-end,..."`
示例: `"1-20,50-60,100-"` (100-表示100到文件末尾)

### 缩进和tab

`indent_encoding=true` 时，行首tab会按 `tab_width` 归一化为空格。
这提升LLM可读性，但不是逐字节可逆转换。
如果必须保留行首tab/space的原始混合形态，请关闭 `indent_encoding`。

### extraction.rules匹配
按列表顺序逐一匹配文件路径(相对于root)。
第一个命中的规则生效。
未命中任何规则的文件使用 `default_mode`。

### partial extraction与apply

`lines` 和 `regex` 模式当前输出的是拼接后的阅读视图，不包含fragment边界和完整原始行号映射。
这些file块会带 `extraction="partial"` 标记。
因此这类文件不保证能安全应用LLM返回的patch/replace。
当前校验会对包含partial extraction的profile给出warning；
需要反向apply的文件应使用 `full` 模式。

### 默认值
未指定的 `global` 字段使用 `GlobalConfig::default()`。
未指定的 profile section 使用对应 section 的默认值；profile 不从 `global` 继承字段。
如果 YAML 省略整个 `profiles` 字段，反序列化时会创建 `default` profile。
如果显式写成 `profiles: {}`，profile 相关命令会报错。
