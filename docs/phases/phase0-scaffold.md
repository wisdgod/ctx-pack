# Phase 0: 项目脚手架

## 目标
项目可编译，CLI可解析，配置可加载。无业务逻辑。

## 产出文件

### Cargo.toml

```toml
[package]
name = "ctx-pack"
version = "0.1.0"
edition = "2021"

[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
anyhow = "1"
thiserror = "2"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

后续Phase按需添加依赖。

### src/main.rs

- 初始化 tracing-subscriber（env-filter, 默认info级别）
- 解析CLI参数
- 加载配置文件（如果存在）
- match子命令分发到stub handler
- 所有handler当前只打印"not implemented"

### src/cli.rs

clap derive API定义:

```
ctx-pack
├── init
├── pack
│   --profile <NAME>     (default: "default")
│   -o <FILE>            (覆盖output.file)
│   --stdin              (合并stdin文件列表)
│   --full               (全量模式)
│   --diff               (增量模式)
│   --auto               (自动判断，默认)
├── apply
│   [FILE]               (可选，默认stdin)
│   --dry-run
├── status
│   --profile <NAME>
├── tree
│   --profile <NAME>
├── prompt
│   --profile <NAME>
├── migrate-prefix
│   <OLD> <NEW>
├── cache
│   ├── clean
│   │   --profile <NAME>
│   └── info
```

### src/config/schema.rs

所有配置项的serde结构体。为每个结构体实现 `Default`。

关键类型:
- `Config` (顶层: global + profiles HashMap)
- `GlobalConfig`
- `Profile`
- `RootEntry` (path + label)
- `DiscoveryConfig`
- `ExtractionConfig`
- `ExtractionRule` (match + mode + mode-specific fields)
- `VersioningConfig`
- `OutputConfig`
- `SizePolicy` (enum)
- `BinaryPolicy` (enum)
- `ExtractionMode` (enum)

size string ("500KB"等) 用自定义deserializer解析为 u64 字节数。

### src/config/validation.rs

配置加载后的语义校验:
- profile名称非空
- roots路径存在（警告，非错误）
- replace_threshold在0.0-1.0
- anchor_interval >= 0
- include/exclude是合法glob
- extraction rules的regex可编译

校验返回 `Vec<ConfigWarning>`，不一定abort。

### src/config/mod.rs

公开接口:
- `load_config(path: Option<&Path>) -> Result<Config>`
  - 如果path=None，在当前目录向上搜索 `ctx-pack.yaml`
  - 找不到则返回全默认Config
- `Config::validate(&self) -> Vec<ConfigWarning>`

## 验证标准

- `cargo build` 通过
- `cargo run -- init` 打印 "not implemented"
- `cargo run -- pack --profile test` 解析参数正确
- 手写一个示例 `ctx-pack.yaml`，load + validate 不panic
- `cargo test` 通过（配置解析的单元测试）

## 测试要求

- 默认配置的序列化/反序列化 round-trip
- size string解析: "500KB" → 512000, "10MB" → 10485760, "1234" → 1234
- 校验: 非法regex → warning, threshold超范围 → warning
