# workspace-dependency-policy Specification

## Purpose

Defines cross-platform dependency hygiene and audit rules enforced via `cargo-deny` across Linux, macOS, and Windows workspace targets so newly unmaintained, unsound, or wildcard dependencies are caught early in CI.

## ADDED Requirements

### Requirement: Cross-platform dependency audit detects newly unmaintained crates
The workspace `cargo-deny` configuration SHALL evaluate `advisories`, `licenses`, `bans`, and `sources` across all workspace members (`eqntide`, `eqntide-editor`, `eqntide-desktop`) and all desktop target triples (`x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`, `x86_64-apple-darwin`, `aarch64-apple-darwin`), SHALL enable `unmaintained` advisory checking rather than disabling it globally, and SHALL restrict any advisory ignores to explicitly documented transitive exceptions.

#### Scenario: Full workspace cargo-deny check passes without suppressing direct unmaintained crates
- **WHEN** `cargo deny --all-features --workspace --locked check advisories licenses bans sources` is executed
- **THEN** the check MUST pass with zero errors, zero direct unmaintained dependencies (`ttf-parser` and `ab_glyph` absent from the workspace graph), and any ignored advisories MUST be limited to documented transitive upstream dependencies

### Requirement: Wildcard dependency specifications are disallowed
Workspace dependency policy SHALL deny wildcard (`*`) version requirements across all manifests via `bans.wildcards = "deny"`.

#### Scenario: Dependency audit enforces non-wildcard versions
- **WHEN** `cargo deny --all-features --workspace --locked check bans` runs against the workspace manifests
- **THEN** all workspace dependencies MUST use explicit version constraints and the `bans` check MUST succeed with zero wildcard errors
