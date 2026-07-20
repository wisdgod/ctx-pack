# ctx-pack Documentation

This directory contains the architecture, protocol notes, and original implementation roadmap for ctx-pack.

本目录包含 ctx-pack 的架构设计、协议说明和原始实现路线图。

## Reading Order

**If you want to understand the project:**

1. [`overview.md`](./overview.md) - What this is, core philosophy, glossary
2. [`architecture.md`](./architecture.md) - Data flow diagrams, module map, key design decisions
3. [`protocol.md`](./protocol.md) - The encoding/versioning/tag system that makes this work

**If you want to understand the output format:**

4. [`formats/output.md`](./formats/output.md) - `.ctx` file structure
5. [`formats/manifest.md`](./formats/manifest.md) - Manifest as structural index
6. [`formats/patch.md`](./formats/patch.md) - Patch/Replace block syntax

**If you want to understand the config:**

7. [`config.md`](./config.md) - Full YAML schema reference

**If you want historical implementation context:**

8. [`phases/phase0-scaffold.md`](./phases/phase0-scaffold.md) through [`phases/phase8-finishing.md`](./phases/phase8-finishing.md) - Sequential implementation plan

## Document Map

```
docs/
├── overview.md            Project positioning, philosophy, glossary
├── architecture.md        Core architecture, data flow, module boundaries
├── protocol.md            Representation protocol: encoding, versioning, tags
├── config.md              YAML configuration schema reference
├── formats/
│   ├── output.md          .ctx output file format specification
│   ├── manifest.md        Manifest file structure
│   └── patch.md           Patch and Replace block format specification
└── phases/
    ├── phase0-scaffold.md   CLI + config + project init
    ├── phase1-encoding.md   TransformStage trait, IndentEncoder, AnchorEncoder, Pipeline
    ├── phase2-discovery.md  File discovery (ignore/glob/stdin) + binary/encoding detection
    ├── phase3-extraction.md Content extraction: full, lines, regex modes
    ├── phase4-pack.md       End-to-end pack --full
    ├── phase5-index.md      Persistent index + snapshot cache
    ├── phase6-versioning.md Diff, patch generation, replace threshold
    ├── phase7-apply.md      Tag scanner, dirty check, fuzzy match, .rej fallback
    └── phase8-finishing.md  migrate-prefix, cache, init, size warnings
```

## For AI Coding Assistants

The phase documents in `phases/` were written as self-contained AI coding prompts. They specify:

- Exact file paths and module structure
- Function signatures and data structures
- Behavioral rules and edge cases
- Test requirements with concrete examples

Current workflow:

1. Read the source tree first; it is the source of truth.
2. Use phase docs for design intent and missing-edge-case context.
3. Run `cargo check`, `cargo fmt --check`, `cargo test`, `cargo clippy -- -D warnings`, and `git diff --check` before accepting changes.

The implementation no longer uses `mod.rs`; top-level modules are `src/<module>.rs` plus `src/<module>/` submodules.

## Design Process

This documentation was produced through a multi-model collaborative design process:

- **[wisdgod](https://github.com/wisdgod)**: Product requirements, use-case expertise, final decisions
- **Claude (Anthropic)**: Lead system design - analysis, decomposition, architecture, protocol specification, implementation planning
- **Gemini (Google)**: Adversarial review - identified gaps in diff-on-encoded-content ordering, apply-side dirty state detection, bidirectional encoding requirement, and reframed the manifest as a structural index with future performance potential rather than passive metadata
- **Codex (OpenAI)**: Repository packaging and documentation structure for publication

The design went through multiple revision cycles incorporating feedback from all parties.
