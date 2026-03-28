# Phase 7: Patch应用

## 目标
`ctx-pack apply` 可以从LLM输出中提取patch/replace并应用到源文件。

## 产出文件

### src/apply/scanner.rs

函数: `scan_blocks(input: &str, prefix: &str) -> Vec<ScannedBlock>`

```rust
pub enum ScannedBlock {
    Patch {
        fid: u32,
        gen: u32,
        pid: u32,
        body: String,
    },
    Replace {
        fid: u32,
        gen: u32,
        body: String,
    },
}
```

从自由文本中提取所有 `<prefix:patch ...>...</prefix:patch>` 
和 `<prefix:replace ...>...</prefix:replace>` 块。

实现策略:
- 用regex匹配开闭标签
- 提取标签属性(fid, gen, pid)
- 提取标签体(body)
- 容错: 忽略解析失败的块，warn并继续

**关键: 不依赖manifest。输入是LLM的自由文本。**

### src/apply/executor.rs

```rust
pub struct ApplyResult {
    pub applied: Vec<AppliedFile>,
    pub rejected: Vec<RejectedFile>,
}

pub struct AppliedFile {
    pub fid: u32,
    pub path: String,
    pub was_dirty: bool,  // 是否经过fuzzy match
}

pub struct RejectedFile {
    pub fid: u32,
    pub path: String,
    pub reason: String,
    pub rej_file: PathBuf,
}
```

函数: `execute_apply(blocks: &[ScannedBlock], index: &IndexState, cache: &SnapshotCache, pipeline: &Pipeline, dry_run: bool) -> Result<ApplyResult>`

对每个block:

**Replace流程:**
1. fid → index查路径
2. decode pipeline: body → raw content
3. 脏检测: hash(当前磁盘文件) vs index.current_hash
   - 不匹配 → warn "file modified externally"
   - replace情况下仍然可以应用(全量替换)，但要warn
4. 写文件(除非dry_run)
5. 更新index: hash, gen, pid=0
6. 存新快照

**Patch流程:**
1. fid → index查路径
2. 读取当前磁盘文件
3. 脏检测: hash比对
4. encode当前内容(用pipeline)
5. 解析patch body中的hunks
6. 对每个hunk:
   a. 找anchor行
   b. 用上下文行精确定位
   c. 应用增删
7. 如果精确定位失败 → fuzzy match:
   a. 在anchor±20行范围搜索上下文行序列
   b. 找到 → 应用 + warn
   c. 找不到 → reject
8. decode pipeline: 编码后结果 → raw content
9. 写文件(除非dry_run)
10. 更新index: hash, pid
11. 存新快照

### src/apply/reject.rs

函数: `write_reject(path: &Path, block: &ScannedBlock) -> Result<PathBuf>`

在目标文件旁写 `{filename}.rej`:
- 包含原始patch/replace块内容
- 包含失败原因注释

### src/apply/mod.rs

apply命令的完整handler:
1. 读取输入(文件或stdin)
2. 加载config获取prefix
3. scan_blocks
4. 加载IndexState和SnapshotCache
5. build_pipeline(config)
6. execute_apply
7. 打印结果摘要
8. 保存IndexState

## Fuzzy Match详细算法

```
给定: hunk的context_before行 + 目标anchor行号N

1. 在encoded文件中找到anchor行N (精确位置)
2. 提取该位置周围的实际行
3. 将hunk的context_before与实际行对比
4. 完全匹配 → 精确定位成功

5. 若不匹配(文件被修改导致偏移):
   for offset in 1..=20:
     检查 N-offset 和 N+offset 位置
     将context与该位置周围的行对比
     匹配 → 返回偏移后的位置

6. 所有偏移都不匹配 → reject
```

## 测试要求

- scanner: 从包含自由文本+patch块的字符串中正确提取
- scanner: 多个块混合提取
- scanner: 格式不完整的块被跳过
- executor: 正常patch应用
- executor: 正常replace应用
- executor: 脏文件检测 + fuzzy match成功
- executor: fuzzy match失败 → .rej文件生成
- executor: dry-run不写文件
- 集成: pack → 修改文件 → 模拟LLM输出 → apply → 验证文件内容
