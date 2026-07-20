# Phase 6: 版本系统

## 目标
`ctx-pack pack --diff` 和 `--auto` 可用。
能生成patch和replace块。

## 新增依赖

```toml
similar = "3"
```

## 产出文件

### src/version/diff.rs

函数: `compute_change_ratio(old: &str, new: &str) -> f64`
- 用similar计算行级diff
- 返回变化行数 / 总行数

函数: `compute_line_diff(old: &str, new: &str) -> Vec<DiffHunk>`

```rust
pub struct DiffHunk {
    pub old_start: u32,   // 在old文本中的起始行，用于anchor定位
    pub removes: Vec<String>,
    pub adds: Vec<String>,
    pub context_before: Vec<String>,  // hunk前的上下文行
    pub context_after: Vec<String>,
}
```

### src/version/patch_gen.rs

函数: `generate_patch(fid: u32, gen: u32, pid: u32, hunks: &[DiffHunk], prefix: &str, anchor_interval: u32) -> String`

将DiffHunk转为anchor-based patch格式:
1. 对每个hunk，找到最近的anchor行号
2. 格式化为 `@@ anchor:N @@` + diff行

### src/version/replace_gen.rs

函数: `generate_replace(fid: u32, gen: u32, encoded_content: &str, prefix: &str) -> String`

简单包装encoded_content为replace标签。

### src/version.rs

核心决策函数:

```rust
pub enum VersionAction {
    Unchanged,
    Patch { pid: u32, content: String },
    Replace { gen: u32, content: String },
}
```

函数: `determine_action(fid: u32, old_encoded: &str, new_encoded: &str, current_gen: u32, current_pid: u32, versioning_config: &VersioningConfig, prefix: &str, anchor_interval: u32) -> VersionAction`

1. 如果old == new → Unchanged
2. 计算change_ratio
3. 如果ratio > threshold 或 current_pid >= max_patches → Replace(gen+1)
4. 否则 → Patch(pid+1)

### 修改 pack::output

新函数: `pack_incremental(config: &Config, profile_name: &str) -> Result<PackOutput>`

流程:
1. 加载IndexState
2. Discovery → 当前文件列表
3. 对每个文件:
   a. 在index中? → 加载上次快照, encode(旧), encode(新), determine_action
   b. 不在index中? → 新文件, file块, gen=0
4. index中有但discovery中无? → 标记inactive (不输出，但可在tree中标注)
5. 组装: prompt + tree + 混合的 file/patch/replace 块
6. 更新index + 存新快照

`pack_auto`: 如果index不存在 → pack_full，否则 → pack_incremental

### CLI: --full/--diff/--auto

- --full → pack_full (重置所有gen=0)
- --diff → pack_incremental
- --auto → pack_auto (默认)

## 测试要求

- change_ratio: 完全相同→0.0, 完全不同→1.0
- patch生成: 简单单hunk, 多hunk
- replace阈值判断
- max_patches_before_replace触发
- 集成: 修改一个文件后re-pack，验证输出包含patch块
