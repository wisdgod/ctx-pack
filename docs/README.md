# ctx-pack Documentation

This directory contains the complete architecture and implementation plan for ctx-pack.

本目录包含 ctx-pack 的完整架构设计与实现计划。

## Reading Order

**If you want to understand the project:**

1. [`overview.md`](./overview.md) - What this is, core philosophy, glossary
2. [`architecture.md`](./architecture.md) - Data flow diagrams, module map, key design decisions
3. [`protocol.md`](./protocol.md) - The encoding/versioning/tag system that makes this work

**If you want to understand the output format:**

4. [`formats/output.md`](./formats/output.md) - `.ctx` file structure
5. [`formats/manifest.md`](./formats/manifest.md) - Manifest as operational index
6. [`formats/patch.md`](./formats/patch.md) - Patch/Replace block syntax

**If you want to understand the config:**

7. [`config.md`](./config.md) - Full YAML schema reference

**If you want to implement:**

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
│   ├── manifest.md        Manifest file: structure, incremental rewrite strategy
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

Each phase document in `phases/` is self-contained enough to be used as an AI coding prompt. They specify:

- Exact file paths and module structure
- Function signatures and data structures
- Behavioral rules and edge cases
- Test requirements with concrete examples

Recommended workflow:

1. Feed the AI the relevant phase document
2. Optionally include `architecture.md` and `protocol.md` for context
3. Review generated code against the spec
4. Run `cargo test` before moving to the next phase

Phase dependencies are linear: each phase builds on the previous. Do not skip phases.

## Design Process

This documentation was produced through a multi-model collaborative design process:

- **[wisdgod](https://github.com/wisdgod)**: Product requirements, use-case expertise, final decisions
- **Claude (Anthropic)**: Lead system design - analysis, decomposition, architecture, protocol specification, implementation planning
- **Gemini (Google)**: Adversarial review - identified gaps in diff-on-encoded-content ordering, apply-side dirty state detection, bidirectional encoding requirement, and reframed the manifest as a performance-critical operational index rather than passive metadata
- **Codex (OpenAI)**: Repository packaging and documentation structure for publication

The design went through multiple revision cycles incorporating feedback from all parties.
