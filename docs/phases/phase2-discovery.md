# Phase 2: 文件发现与检测

## 目标
给定配置的discovery规则，产出合格文件路径列表（已过滤二进制，已转UTF-8）。

## 新增依赖

```toml
ignore = "0.4"
globset = "0.4"
content_inspector = "0.2"
encoding_rs = "0.8"
```

## 产出文件

### src/discovery/builtin.rs

函数: `discover_builtin(profile: &Profile) -> Result<Vec<PathBuf>>`

流程:
1. 对profile.roots中每个root，构建 `ignore::WalkBuilder`
2. 设置 use_gitignore = profile.discovery.use_gitignore
3. Walk收集所有文件路径
4. 用 `globset` 构建include matcher和exclude matcher
5. 过滤: include匹配 AND NOT exclude匹配
6. 路径归一化: 相对于对应root的相对路径，保留root label前缀
   如 root={path:"web/src", label:"frontend"} → "frontend/components/App.tsx"

### src/discovery/stdin.rs

函数: `discover_stdin() -> Result<Vec<PathBuf>>`

从stdin逐行读取路径。跳过空行和#开头的注释行。
验证路径存在，不存在的路径发warning。

### src/discovery.rs

函数: `discover(profile: &Profile) -> Result<Vec<DiscoveredFile>>`

```rust
pub struct DiscoveredFile {
    pub absolute_path: PathBuf,
    pub display_path: String,  // 归一化的显示路径(相对于root+label)
}
```

1. 调用builtin
2. 如果stdin_merge=true，调用stdin，合并
3. 去重（按absolute_path）
4. 按display_path排序

### src/detection/binary.rs

函数: `is_binary(path: &Path) -> Result<bool>`

读取前8192字节，用 `content_inspector::inspect` 判断。

### src/detection/encoding.rs

函数: `read_to_utf8(path: &Path) -> Result<String>`

1. 读取文件全部字节
2. 如果是valid UTF-8 → 直接返回String
3. 否则用 encoding_rs 检测编码
4. 转换为UTF-8
5. 转换失败 → 返回错误

### src/detection.rs

函数: `load_file_content(path: &Path, binary_policy: BinaryPolicy) -> Result<Option<String>>`

1. is_binary检查
   - 是二进制 + skip → return Ok(None)
   - 是二进制 + warn → warn + return Ok(None)
   - 是二进制 + abort → return Err
2. read_to_utf8

## 测试要求

- builtin: 创建temp目录结构，验证glob过滤
- stdin: 模拟stdin输入（用测试helper）
- binary: 准备一个含NUL字节的文件 + 一个纯文本文件
- encoding: 准备一个UTF-8文件 + 一个非UTF-8文件（如Latin-1）
- 集成: discovery + detection 联合测试
