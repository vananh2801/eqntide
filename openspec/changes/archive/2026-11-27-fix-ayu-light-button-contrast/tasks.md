# Tasks

## 1. Light-Mode Ayu Neutral Palette & Unit Tests

- [x] 1.1 Update Light-mode neutral surface, control, and selection tokens (`app_bg`, `panel_bg`, `card_bg`, `extreme_bg`, `control_bg`, `control_hover_bg`, `control_active_bg`, `control_disabled_bg`, `selected_bg`, `selected_hover_bg`, `selected_fg`, plus Light-mode adaptive `selected_*` lightness) in `desktop/src/theme.rs` per `design.md` while keeping Dark mode unchanged, and verify `cargo test -p eqntide-desktop theme::tests` passes.
- [x] 1.2 Add assertions in `desktop/src/theme.rs` unit tests verifying Light-mode surface/control/selection luminance direction (`app_bg > panel_bg > card_bg > control_bg > control_hover_bg > control_active_bg` and `card_bg > selected_bg > selected_hover_bg` in relative luminance) and Dark-mode luminance direction (`control_bg` brighter than `card_bg` / `panel_bg` / `app_bg`, and `control_bg < control_hover_bg < control_active_bg`), and verify `cargo test -p eqntide-desktop theme::tests` passes.

## 2. Desktop UI & Pre-Commit Verification

- [x] 2.1 Run desktop layout and batch/settings theme test suites (`cargo test -p eqntide-desktop`) and verify all Light/Dark contrast, borderless chrome, progress-bar, and popup checks pass without regressions.
- [x] 2.2 Run full workspace pre-commit checks (`cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --all-targets --locked`, and `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`) and verify zero warnings or failures.
