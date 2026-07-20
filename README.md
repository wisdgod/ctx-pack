# ctx-pack

**A configuration-driven source file normalization tool that packs codebase context into LLM-ready structured files with a built-in versioning protocol for incremental patch/replace.**

**配置驱动的源文件归一化工具，将代码库内容打包为LLM可读的结构化上下文文件，内建版本协议支持增量patch/replace。**

---

## Why / 为什么

Feeding code to LLMs is deceptively hard. You need to select files, format them readably, track changes across conversation turns, and apply LLM-generated modifications back to your codebase. Existing tools (`repomix`, `code2prompt`) handle the simple case but lack versioning, incremental updates, and bidirectional patch support.

把代码喂给LLM看似简单实则困难。你需要选择文件、格式化为可读形式、在对话轮次间跟踪变化、并将LLM生成的修改应用回代码库。现有工具处理简单场景足够，但缺乏版本管理、增量更新和双向patch支持。

ctx-pack treats LLM context as a **representation protocol**, not a flat dump:

ctx-pack 将LLM上下文视为一套**表征协议**，而非简单的文件拼接：

- **Token-efficient encoding** — absolute indent encoding (`[N]` prefix), anchor line numbers (every N lines)
- **Version addressing** — `(fid, gen, pid)` model: file ID + generation (replace) + patch ID
- **Bidirectional** — pack files for LLM consumption, apply LLM patches back to source
- **Incremental context** — persistent file index, snapshot cache, patch/replace output for changed files
- **Fault-tolerant apply** — anchor-based patch format + fuzzy match, because LLMs are imprecise and the tool should adapt
- **Self-describing** — auto-generated prompt header explains the protocol to the LLM dynamically based on your config
- **Config-driven** — YAML profiles, nested extraction rules, every behavior is configurable

- **Token高效编码** — 绝对缩进编码（`[N]` 前缀）、锚定行号（每N行）
- **版本寻址** — `(fid, gen, pid)` 模型：文件编号 + 代号(replace) + 补丁序号
- **双向操作** — 打包文件供LLM阅读，应用LLM补丁回源文件
- **上下文增量** — 持久化文件索引、快照缓存、变更文件的patch/replace输出
- **容错应用** — 基于锚点的patch格式 + 模糊匹配，因为LLM不精确，工具应当适应
- **协议自描述** — 根据配置动态生成prompt头部，向LLM解释协议
- **配置驱动** — YAML多profile、嵌套提取规则、一切行为可配置

## Status / 状态

✅ **Core implementation complete. Hardening and protocol polish in progress.**

✅ **核心实现已完成，正在继续完善边界行为和协议细节。**

The implementation currently supports init, full/diff/auto pack, apply, status, tree, prompt, cache cleanup, and prefix migration. The phase documents in [`docs/phases/`](./docs/phases/) are retained as implementation-roadmap history; the source tree and this README are the current status reference.

当前实现支持 init、full/diff/auto pack、apply、status、tree、prompt、cache clean 和 prefix 迁移。[`docs/phases/`](./docs/phases/) 中的阶段文档保留为实现路线图历史；当前状态以源码和本 README 为准。

Apply is intended for files packed with `full` extraction. `lines` and `regex` extraction produce file blocks marked `extraction="partial"`; these are read-only context views and do not carry enough fragment mapping to guarantee safe reverse application. With `indent_encoding=true`, leading tabs are normalized to spaces according to `tab_width`.

Apply 面向 `full` extraction 打包的文件。`lines` 和 `regex` extraction 产出带 `extraction="partial"` 标记的只读上下文视图，不携带足够的片段映射信息来保证安全反向应用。启用 `indent_encoding=true` 时，行首 tab 会按 `tab_width` 归一化为空格。

## Quick Example / 快速示例

```yaml
# ctx-pack.yaml
global:
  prefix: "ctx"
  anchor_interval: 10
  indent_encoding: true

profiles:
  default:
    roots:
      - path: "."
        label: "project"
    discovery:
      use_gitignore: true
      include: ["**/*.rs"]
      exclude: ["target/**"]
    extraction:
      default_mode: full
    output:
      file: context.ctx
```

```bash
# Pack all matching files
ctx-pack pack --full

# After editing source files, generate incremental update
ctx-pack pack --diff

# Apply LLM-generated patches back
ctx-pack apply llm_response.txt

# Preview what the LLM will see
ctx-pack prompt
```

Output format:

```xml
<ctx:prompt>
This context uses absolute indent encoding: [N] means N leading spaces.
Line numbers are anchored every 10 lines. ...
</ctx:prompt>

<ctx:tree>
[1] project/src/main.rs (gen0)
[2] project/src/lib.rs (gen0)
</ctx:tree>

<ctx:file id="1" gen="0" path="project/src/main.rs">
   1 | [0]fn main() {
     | [4]let config = load();
     | [4]run(config);
  10 | [0]}
</ctx:file>

<ctx:patch fid="2" gen="0" pid="1">
@@ anchor:10 @@
-[4]old_function();
+[4]new_function();
</ctx:patch>
```

## Architecture Overview / 架构概览

```
Pack:  Config → Discovery → Detection → Extraction → Encode Pipeline → Version Diff → Output + Manifest
Apply: LLM Text → Tag Scanner → Dirty Check → Fuzzy Patch/Replace → Decode Pipeline → Write Files
```

Key subsystems / 关键子系统:

| Subsystem | Purpose |
|-----------|---------|
| **Encoding Layer** | Bidirectional transform pipeline (`encode`/`decode`), extensible via `TransformStage` trait |
| **Version System** | `(fid, gen, pid)` addressing, diff against last snapshot, threshold-based patch vs replace |
| **Manifest** | Byte-level block index of generated output, written alongside `.ctx` for inspection/tooling |
| **Apply Engine** | Regex-based tag scanner, hash-based dirty detection, fuzzy patch with `.rej` fallback |
| **Prompt Generator** | Config-aware dynamic protocol description, embedded in output for LLM self-guidance |

## Development Guide / 开发指南

### Prerequisites / 前置条件

- Rust 1.97+ (early CLI baseline; may track current stable before initial release)
- Cargo

### Install / 安装

```bash
# The crate is published as `ctxpack`; the installed binary is `ctx-pack`.
cargo install ctxpack
```

### Build / 构建

```bash
git clone https://github.com/wisdgod/ctx-pack.git
cd ctx-pack
cargo build
cargo test
```

### Project Structure / 项目结构

```
ctx-pack/
├── docs/                    # Architecture & implementation plans
│   ├── README.md            # Reading guide and AI assistant usage
│   ├── overview.md          # Project positioning, philosophy, glossary
│   ├── architecture.md      # Data flow, module boundaries, key decisions
│   ├── protocol.md          # Encoding, versioning, tag system specs
│   ├── config.md            # YAML config schema reference
│   ├── formats/             # Output file format specs
│   │   ├── output.md        # .ctx file structure
│   │   ├── manifest.md      # Manifest file structure
│   │   └── patch.md         # Patch/Replace block syntax
│   └── phases/              # Implementation roadmap (8 phases)
│       ├── phase0-scaffold.md
│       ├── phase1-encoding.md
│       ├── phase2-discovery.md
│       ├── phase3-extraction.md
│       ├── phase4-pack.md
│       ├── phase5-index.md
│       ├── phase6-versioning.md
│       ├── phase7-apply.md
│       └── phase8-finishing.md
├── src/                     # Rust 2024 implementation
│   ├── main.rs
│   ├── cli.rs
│   ├── config.rs            # top-level module files; no mod.rs layout
│   ├── config/
│   ├── discovery.rs
│   ├── discovery/
│   └── ...
├── Cargo.toml
├── rust-toolchain.toml
├── rustfmt.toml
└── clippy.toml
```

### Implementation Roadmap / 实现路线图

The project was implemented from an 8-phase roadmap, each phase producing compilable, testable output:

项目从8个阶段路线图实现，每阶段产出可编译、可测试的成果：

| Phase | Focus | Key Output |
|-------|-------|------------|
| 0 | Scaffold | CLI (clap), config (serde + YAML), project structure |
| 1 | Encoding Layer | `TransformStage` trait, `IndentEncoder`, `AnchorEncoder`, `Pipeline` - pure functions, heavy unit tests |
| 2 | File Discovery | `ignore` + `globset` + stdin, binary detection (`content_inspector`), encoding conversion (`encoding_rs`) |
| 3 | Content Extraction | Full / lines / regex modes, fragment model, first-match-wins rule matcher |
| 4 | Basic Pack | End-to-end `ctx-pack pack --full`, tree + file blocks + prompt + manifest |
| 5 | Index & Cache | Persistent FID allocation, SHA-256 hash tracking, snapshot cache with auto-eviction |
| 6 | Versioning | `similar` crate diff, patch/replace generation, `--diff` and `--auto` modes |
| 7 | Apply | Tag scanner, dirty detection, fuzzy match, `.rej` fallback, decode pipeline |
| 8 | Finishing | `migrate-prefix`, `cache clean`, `init`, size warnings |

Each phase document in `docs/phases/` contains the original implementation prompt: exact file paths, function signatures, data structures, and test requirements. These files are useful for design context, but they may lag behind the current source layout and implementation details.

每个阶段文档包含原始实现提示词：精确的文件路径、函数签名、数据结构和测试要求。这些文件适合了解设计背景，但可能落后于当前源码布局和实现细节。

### Contributing / 贡献

1. Read `docs/overview.md` and `docs/architecture.md` for context
2. Pick a phase from `docs/phases/`
3. Implement according to the spec
4. Ensure `cargo test` passes
5. Submit a PR

Design discussions and improvements to the plan itself are also welcome - open an issue.

1. 阅读 `docs/overview.md` 和 `docs/architecture.md` 了解背景
2. 从 `docs/phases/` 中选择一个阶段
3. 按规范实现
4. 确保 `cargo test` 通过
5. 提交PR

也欢迎对设计本身的讨论和改进 - 请开issue。

## Design Credits / 设计致谢

The architecture and implementation plan were designed collaboratively:

架构和实现计划由以下协作设计：

- **[wisdgod](https://github.com/wisdgod)** — Product vision, requirements, key decisions
- **Claude (Anthropic)** - Lead architect: analysis, system design, protocol specification, implementation plans
- **Gemini (Google)** - Design review: identified critical issues in diff ordering, apply conflict handling, encoding bidirectionality, and manifest's role as structural index with future performance potential
- **Codex (OpenAI)** — Repository packaging: README, LICENSE, and documentation structure for publication
- **[wisdgod](https://github.com/wisdgod)** — 产品构想、需求定义、关键决策
- **Claude (Anthropic)** - 主架构师：分析、系统设计、协议规范、实现计划
- **Gemini (Google)** - 设计评审：识别了diff顺序、apply冲突处理、编码双向性、清单文件作为结构索引及后续性能优化基础等关键问题
- **Codex (OpenAI)** — 仓库封装：README、LICENSE及发布用文档结构整理

## License / 许可

[MIT](./LICENSE)
