# 核心架构

## 数据流

### Pack方向

```
Config(YAML)
    │
    ▼
Discovery ──→ 文件路径列表
(ignore + glob + stdin合并去重, 路径归一化)
    │
    ▼
Detection ──→ 过滤二进制, 检测编码, 转UTF-8
(content_inspector + encoding_rs)
    │
    ▼
Extraction ──→ 文本片段
(full / lines / regex, first-match-wins规则匹配)
    │
    ▼ raw content
Encode Pipeline ──→ encoded content
(indent_encode → anchor_insert)
    │
    ├── 首次(无快照): 存raw快照, 输出 <prefix:file>
    │
    └── 非首次: 加载上次raw快照
               用当前配置提取并encode旧快照
               diff(strip_anchor_margin(old_encoded), strip_anchor_margin(new_encoded))
                 │
                 ├── 小变化 → <prefix:patch>, pid += 1
                 └── 大变化 → <prefix:replace>, gen += 1, pid = 0
                 │
                 存新raw快照, 更新索引
    │
    ▼
Pack Engine ──→ 组装输出
(prompt_header + tree + content blocks)
    │
    ├──→ 主文件 (.ctx)
    └──→ 清单文件 (.ctx.manifest)
```

### Apply方向

```
LLM自由文本
    │
    ▼
Tag Scanner ──→ 提取 patch/replace 块
(regex扫描, 从混乱文本中健壮提取)
    │
    ▼ 对每个块:
Index Lookup ──→ fid → 文件路径
    │
    ▼
Version Check ──→ 防止stale gen/pid写入
    │
    ▼
Dirty Check ──→ hash(磁盘文件) vs index.current_hash
    │
    ├── 匹配: 正常apply
    ├── patch不匹配: fuzzy match尝试
    │             │
    │             ├── 成功: apply + 警告
    │             └── 失败: 写 .rej 文件, 跳过
    └── replace不匹配: 写 .rej 文件, 跳过
    │
    ▼
Patch: Content Pipeline(无anchor栏) encode磁盘文件 → apply hunk → Content Pipeline decode
Replace: Full Pipeline反序(anchor_strip → indent_decode) ──→ raw content
    │
    ▼
Write File ──→ 更新索引hash + 存新快照
```

## 模块边界

```
src/
├── main.rs                  # 入口: tracing初始化, CLI dispatch
├── cli.rs                   # clap定义
├── config.rs                # 统一接口: load/validate/re-export
├── config/
│   ├── schema.rs            # serde结构体
│   └── validation.rs        # 语义校验
├── discovery.rs             # 统一接口: discover(profile) -> Vec<DiscoveredFile>
├── discovery/
│   ├── builtin.rs           # ignore + globset
│   └── stdin.rs             # stdin行读取
├── detection.rs             # 统一接口: load_file_content
├── detection/
│   ├── binary.rs            # content_inspector
│   └── encoding.rs          # encoding_rs → UTF-8
├── extraction.rs            # 统一接口: extract/match_rule
├── extraction/
│   ├── full.rs
│   ├── lines.rs
│   └── regex_extract.rs
├── encoding_layer.rs        # build_pipeline
├── encoding_layer/
│   ├── traits.rs            # TransformStage trait
│   ├── indent.rs            # IndentEncoder
│   ├── anchor.rs            # AnchorEncoder
│   └── pipeline.rs          # Pipeline: 有序stage链
├── index.rs                 # hash helper
├── index/
│   ├── state.rs             # 索引读写
│   ├── fid.rs               # FID分配
│   └── cache.rs             # 快照缓存管理 + 自动淘汰
├── version.rs               # patch/replace决策
├── version/
│   ├── diff.rs              # similar集成, 行级diff
│   ├── patch_gen.rs         # 生成patch块
│   └── replace_gen.rs       # 生成replace块, 阈值判断
├── pack.rs                  # CLI handler
├── pack/
│   ├── tree.rs              # tree索引生成
│   ├── output.rs            # 主文件组装
│   ├── manifest.rs          # 清单文件生成
│   └── prompt.rs            # 协议自描述prompt生成
├── apply.rs                 # CLI handler
├── apply/
│   ├── scanner.rs           # tag scanner, 从自由文本提取块
│   ├── executor.rs          # patch/replace执行 + 脏检测 + fuzzy
│   └── reject.rs            # .rej文件生成
├── migrate.rs               # CLI handler
└── migrate/
    └── prefix.rs            # 标签前缀迁移
```

## 关键设计决策

### 清单文件记录输出结构

清单精确记录输出文件中每个块的字节/行偏移，并记录标签出现位置。
当前实现每次重新生成输出文件，再写出新的manifest。
局部重写仍是可基于manifest实现的后续优化，不是当前行为。

### Diff基于编码后的内容投影

LLM的认知基准是编码后文本（带[N]前缀和anchor行号）。
Replace中的内容必须与LLM看到的file块内部格式一致。
Patch hunk则使用去掉左侧anchor行号栏后的内容行，避免要求LLM复制空白对齐和 `|` 分隔符。

因此patch diff流程: raw快照 → 用当前配置extract → encode → strip_anchor_margin → diff。
快照存raw是因为编码配置可能变化。

### Fuzzy Match是简化Patch格式的基石

我们使用 `@@ anchor:N @@` 而非精确行号偏移，
因为apply端有fuzzy match兜底。
当anchor行号因文件修改而偏移几行时，fuzzy match通过上下文行匹配定位。
这让LLM不必精确计算行号——降低LLM出错概率。

### Partial Extraction是阅读视图

lines/regex模式当前会拼接fragment且不输出边界。
这种视图适合减少上下文体积，但不携带足够信息来可靠反向apply到完整源文件。
输出会在file块上标记 `extraction="partial"`，prompt会要求LLM不要为这类文件生成patch/replace。
需要LLM返回可应用修改的文件，应使用full extraction。
