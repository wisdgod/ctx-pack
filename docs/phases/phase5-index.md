# Phase 5: 索引与缓存

## 目标
持久化文件状态，支持变化检测。

## 新增依赖

```toml
sha2 = "0.10"
chrono = { version = "0.4", features = ["serde"] }
```

## 产出文件

### src/index/state.rs

```rust
pub struct IndexState {
    pub version: u32,
    pub files: HashMap<String, FileEntry>,  // display_path → entry
    pub next_fid: u32,
}

pub struct FileEntry {
    pub fid: u32,
    pub current_hash: String,
    pub current_gen: u32,
    pub current_pid: u32,
    pub status: FileStatus,
    pub first_seen: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

pub enum FileStatus { Active, Inactive }
```

函数:
- `IndexState::load(path: &Path) -> Result<Self>` (YAML反序列化, 不存在则返回空)
- `IndexState::save(&self, path: &Path) -> Result<()>`

### src/index/fid.rs

在IndexState上的方法:
- `allocate_fid(&mut self, display_path: &str) -> u32`
  - 已存在(含inactive) → 返回已有fid，如inactive则reactivate
  - 不存在 → 分配next_fid，next_fid += 1
- `deactivate(&mut self, display_path: &str)`

### src/index/cache.rs

快照缓存管理。

```rust
pub struct SnapshotCache {
    pub cache_dir: PathBuf,
}
```

方法:
- `store_snapshot(&self, fid: u32, gen: u32, raw_content: &str) -> Result<()>`
  写入 `{cache_dir}/snapshots/{fid}/gen{gen}.raw`
- `load_snapshot(&self, fid: u32, gen: u32) -> Result<Option<String>>`
- `cleanup(&self, fid: u32, retain_last_n: u32) -> Result<()>`
  保留最近N个gen，删除更早的
- `cleanup_all(&self, index: &IndexState, retain_last_n: u32) -> Result<()>`
  对所有active文件执行cleanup

### src/index/mod.rs

hash计算函数: `compute_hash(content: &str) -> String`
返回 "sha256:{hex}"

### 集成到pack

修改 `pack::output::pack_full`:
1. 加载IndexState
2. 对每个文件: allocate_fid, compute_hash, store_snapshot
3. Pack完成后save IndexState

### CLI: status命令

`ctx-pack status --profile NAME`:
1. Discovery获取当前文件列表
2. 加载IndexState
3. 对比:
   - 新文件(不在index中)
   - 删除的文件(在index中但不在discovery中)
   - 修改的文件(hash不同)
   - 未变化的文件
4. 打印摘要

## 测试要求

- FID分配: 新文件得到递增ID
- FID分配: inactive文件复活保持原ID
- 快照存储/加载 round-trip
- 快照cleanup: 保留最近3个，删除更早的
- hash计算确定性
- 索引序列化/反序列化 round-trip
