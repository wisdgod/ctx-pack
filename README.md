# ctx-pack

**A configuration-driven source file normalization tool that packs codebase context into LLM-ready structured files with a built-in versioning protocol for incremental patch/replace.**

**配置驱动的源文件归一化工具，将代码库内容打包为LLM可读的结构化上下文文件，内建版本协议支持增量patch/replace。**

---

## Why / 为什么

Feeding code to LLMs is deceptively hard. You need to select files, format them readably, track changes across conversation turns, and apply LLM-generated modifications back to your codebase. Existing tools (`repomix`, `code2prompt`) handle the simple case but lack versioning, incremental updates, and bidirectional patch support.

把代码喂给LLM看似简单实则困难。你需要选择文件、格式化为可读形式、在对话轮次间跟踪变化、并将LLM生成的修改应用回代码库。现有工具处理简单场景足够，但缺乏版本管理、增量更新和双向patch支持。

ctx-pack treats LLM context as a **representation protocol**, not a flat dump:

ctx-pack 将LLM上下文视为一套**表征协议**，而非简单的文件拼接：

- **Token-efficient encoding** — absolute indent encoding (`[N]` prefix), anchor line numbers (every N lines), base indent stripping
- **Version addressing** — `(fid, gen, pid)` model: file ID + generation (replace) + patch ID
- **Bidirectional** — pack files for LLM consumption, apply LLM patches back to source
- **Incremental everything** — persistent file index, snapshot cache, manifest-driven partial output rewrite
- **Fault-tolerant apply** — anchor-based patch format + fuzzy match, because LLMs are imprecise and the tool should adapt
- **Self-describing** — auto-generated prompt header explains the protocol to the LLM dynamically based on your config
- **Config-driven** — YAML profiles, nested extraction rules, every behavior is configurable

- **Token高效编码** — 绝对缩进编码（`[N]` 前缀）、锚定行号（每N行）、公共缩进剥离
- **版本寻址** — `(fid, gen, pid)` 模型：文件编号 + 代号(replace) + 补丁序号
- **双向操作** — 打包文件供LLM阅读，应用LLM补丁回源文件
- **全面增量** — 持久化文件索引、快照缓存、清单驱动的局部输出重写
- **容错应用** — 基于锚点的patch格式 + 模糊匹配，因为LLM不精确，工具应当适应
- **协议自描述** — 根据配置动态生成prompt头部，向LLM解释协议
- **配置驱动** — YAML多profile、嵌套提取规则、一切行为可配置

## Status / 状态

🚧 **Design phase complete. Implementation not started.**

🚧 **设计阶段完成，尚未开始实现。**

The full architecture and implementation plan are documented in [`docs/`](./docs/). Contributions welcome.

完整的架构和实现计划已记录在 [`docs/`](./docs/) 中。欢迎贡献。

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
[1] src/main.rs (gen0)
[2] src/lib.rs (gen0)
</ctx:tree>

<ctx:file id="1" gen="0" path="src/main.rs">
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
| **Manifest** | Byte-level block index of output file, enables incremental partial rewrite |
| **Apply Engine** | Regex-based tag scanner, hash-based dirty detection, fuzzy match with `.rej` fallback |
| **Prompt Generator** | Config-aware dynamic protocol description, embedded in output for LLM self-guidance |

## Development Guide / 开发指南

### Prerequisites / 前置条件

- Rust 1.75+ (2021 edition)
- Cargo

### Build / 构建

```bash
git clone https://github.com/<owner>/ctx-pack.git
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
│   │   ├── manifest.md      # Manifest file structure & incremental rewrite
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
├── src/                     # (not yet implemented)
└── Cargo.toml               # (not yet created)
```

### Implementation Roadmap / 实现路线图

The project is designed to be implemented in 8 phases, each producing compilable, testable output:

项目设计为8个阶段实现，每阶段产出可编译、可测试的成果：

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

Each phase document in `docs/phases/` contains exact file paths, function signatures, data structures, and test requirements. They are designed to be directly consumable by AI coding assistants (Codex, Claude, etc.).

每个阶段文档包含精确的文件路径、函数签名、数据结构和测试要求，可直接交给AI编程助手执行。

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
- **Gemini (Google)** - Design review: identified critical issues in diff ordering, apply conflict handling, encoding bidirectionality, and manifest's role as operational index
- **Codex (OpenAI)** — Repository packaging: README, LICENSE, and documentation structure for publication
- **[wisdgod](https://github.com/wisdgod)** — 产品构想、需求定义、关键决策
- **Claude (Anthropic)** - 主架构师：分析、系统设计、协议规范、实现计划
- **Gemini (Google)** - 设计评审：识别了diff顺序、apply冲突处理、编码双向性、清单文件作为可操作索引等关键问题
- **Codex (OpenAI)** — 仓库封装：README、LICENSE及发布用文档结构整理

## License / 许可

[MIT](./LICENSE)
