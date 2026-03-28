# 配置规范

配置文件: `ctx-pack.yaml`，位于项目根目录。

## 完整Schema

```yaml
global:
  prefix: "ctx"                  # string, 标签前缀
  anchor_interval: 10            # u32, 锚定行号间隔, 0=禁用
  indent_encoding: true          # bool, 缩进编码开关
  tab_width: 4                   # u32, tab转空格数
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

### extraction.rules匹配
按列表顺序逐一匹配文件路径(相对于root)。
第一个命中的规则生效。
未命中任何规则的文件使用 `default_mode`。

### 默认值
未指定的profile字段继承global中的对应值（如果存在）。
global本身有硬编码默认值（见schema.rs中的Default实现）。
