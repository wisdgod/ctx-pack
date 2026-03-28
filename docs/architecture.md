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
               用当前配置encode旧快照
               diff(old_encoded, new_encoded)
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
Dirty Check ──→ hash(磁盘文件) vs index.current_hash
    │
    ├── 匹配: 正常apply
    └── 不匹配: fuzzy match尝试
                 │
                 ├── 成功: apply + 警告
                 └── 失败: 写 .rej 文件, 跳过
    │
    ▼
Decode Pipeline (反序) ──→ raw content
(anchor_strip → indent_decode)
    │
    ▼
Write File ──→ 更新索引hash + 存新快照
```

## 模块边界

```
src/
├── main.rs                  # 入口: tracing初始化, CLI dispatch
├── cli.rs                   # clap定义
├── config/
│   ├── mod.rs
│   ├── schema.rs            # serde结构体
│   └── validation.rs        # 语义校验
├── discovery/
│   ├── mod.rs               # 统一接口: discover(config) -> Vec<FilePath>
│   ├── builtin.rs           # ignore + globset
│   └── stdin.rs             # stdin行读取
├── detection/
│   ├── mod.rs               # 统一接口: detect(path) -> FileType
│   ├── binary.rs            # content_inspector
│   └── encoding.rs          # encoding_rs → UTF-8
├── extraction/
│   ├── mod.rs               # 统一接口: extract(content, rules) -> Vec<Fragment>
│   ├── full.rs
│   ├── lines.rs
│   └── regex.rs
├── encoding_layer/
│   ├── mod.rs
│   ├── traits.rs            # TransformStage trait
│   ├── indent.rs            # IndentEncoder
│   ├── anchor.rs            # AnchorEncoder
│   └── pipeline.rs          # Pipeline: 有序stage链
├── index/
│   ├── mod.rs               # 统一接口: load/save/update
│   ├── state.rs             # 索引读写
│   ├── fid.rs               # FID分配
│   └── cache.rs             # 快照缓存管理 + 自动淘汰
├── version/
│   ├── mod.rs
│   ├── diff.rs              # similar集成, 行级diff
│   ├── patch_gen.rs         # 生成patch块
│   └── replace_gen.rs       # 生成replace块, 阈值判断
├── pack/
│   ├── mod.rs               # 统一接口: pack(config, profile) -> Output
│   ├── tree.rs              # tree索引生成
│   ├── output.rs            # 主文件组装
│   ├── manifest.rs          # 清单文件生成+增量重写
│   └── prompt.rs            # 协议自描述prompt生成
├── apply/
│   ├── mod.rs               # 统一接口: apply(input) -> Results
│   ├── scanner.rs           # tag scanner, 从自由文本提取块
│   ├── executor.rs          # patch/replace执行 + 脏检测 + fuzzy
│   └── reject.rs            # .rej文件生成
├── migrate/
│   └── prefix.rs            # 标签前缀迁移
└── warning/
    └── size.rs              # 大小检查
```

## 关键设计决策

### 清单文件是可操作索引

清单不仅记录元数据。它精确记录输出文件中每个块的字节/行偏移。
当只有少数文件变化时，pack可以利用manifest直接seek到对应位置，
局部重写输出文件，而非每次重新生成整个文件。
这是处理大代码库时的核心性能设计。

### Diff基于编码后内容

LLM的认知基准是编码后文本（带[N]前缀和anchor行号）。
Patch中的内容必须与LLM看到的一致。
因此diff流程: raw快照 → 用当前配置encode → diff(旧encoded, 新encoded)。
快照存raw是因为编码配置可能变化。

### Fuzzy Match是简化Patch格式的基石

我们使用 `@@ anchor:N @@` 而非精确行号偏移，
因为apply端有fuzzy match兜底。
当anchor行号因文件修改而偏移几行时，fuzzy match通过上下文行匹配定位。
这让LLM不必精确计算行号——降低LLM出错概率。
