# Proposal

## Why

The workspace currently relies on `ttf-parser` (direct dependency) and `ab_glyph` (which pulls `owned_ttf_parser` and `ttf-parser`) for bundled STIX font metadata and outline extraction, while `deny.toml` suppresses unmaintained-crate checks globally (`unmaintained = "none"`) and permits wildcard dependencies (`wildcards = "allow"`). For long-term cross-platform maintenance across Linux, macOS, and Windows, the project needs an actively maintained font-parsing stack (`read-fonts` + `skrifa`) and a strict `cargo-deny` policy that surfaces any newly unmaintained dependency without silently hiding transitive Linux accessibility crates.

## What Changes

- Replace `ttf-parser` and `ab_glyph` in the core `eqntide` crate with `skrifa` and `read-fonts` (0.47+, `MIT OR Apache-2.0`) while retaining zero-dependency `ab_glyph_rasterizer` for PNG coverage rasterization.
- Migrate OpenType `MATH` table parsing, `GSUB` `ssty` optical-size mapping, glyph lookup, horizontal metrics, bounding boxes, and glyph outline extraction in `src/render/font.rs` and `src/render/font/*` to `read-fonts` / `skrifa`.
- Expand font & rendering unit tests to verify OpenType `MATH` constants, `ssty` alternates, bounding boxes, outline curve segments, and SVG/PNG/EMF output parity.
- Tighten `deny.toml` to set `unmaintained = "all"` (ignoring only the three transitive Linux `zbus` 5.x accessibility split-crate advisories `RUSTSEC-2026-0370`, `RUSTSEC-2026-0371`, `RUSTSEC-2026-0372` until upstream `accesskit`/`eframe` upgrades to `zbus 6+`) and `wildcards = "deny"`, while preserving all four Linux, Windows, and macOS target triples in `[graph].targets`.

## Capabilities

### New Capabilities
- `math-font-backend`: Bundled STIX font loading, OpenType `MATH`/`GSUB` extraction, glyph selection, and outline/polygon rendering backed by actively maintained font crates (`read-fonts` and `skrifa`).
- `workspace-dependency-policy`: Cross-platform `cargo-deny` audit policy enforcing `unmaintained` visibility, explicit scoped advisory exceptions, wildcard bans, and Linux/macOS/Windows target coverage.

### Modified Capabilities
<!-- No existing spec requirements change; conversion/rendering outputs remain backward-compatible. -->

## Impact

- **Dependencies**: [Cargo.toml](Cargo.toml), `Cargo.lock` (adds direct `skrifa` / `read-fonts` >= 0.47; removes `ttf-parser` and `ab_glyph` / `owned_ttf_parser` from the dependency graph).
- **Rendering / Font Module**: [src/render/font.rs](src/render/font.rs), [src/render/font/metadata.rs](src/render/font/metadata.rs), [src/render/font/geometry.rs](src/render/font/geometry.rs), [src/render/font/selection.rs](src/render/font/selection.rs), and [src/render/mod.rs](src/render/mod.rs) (only if rasterization helper signatures adjust).
- **Policy & CI**: [deny.toml](deny.toml) checked by `cargo deny --all-features --workspace --locked check advisories licenses bans sources` in [.github/workflows/ci.yml](.github/workflows/ci.yml).
