# Tasks

## 1. Remove Unused Tracked Assets and Obsolete Validation Log

- [x] 1.1 Remove unused font files (`assets/fonts/stixfonts-2.13b171/STIXTwoText-BoldItalic.ttf`, `OFL-FAQ.txt`; `assets/fonts/JetBrainsMono-2.304/JetBrainsMono-Bold.ttf`, `JetBrainsMono-BoldItalic.ttf`, `JetBrainsMono-Italic.ttf`; `assets/fonts/Nunito/Nunito-Bold.ttf`, `Nunito-BoldItalic.ttf`, `Nunito-Italic.ttf`, `README.txt`) while keeping the 6 embedded `.ttf` fonts, `OFL.txt`, `AUTHORS.txt`, and both STIX `.pdf` reference files, then verify `cargo check --workspace --all-targets` succeeds.
- [x] 1.2 Remove unused WPS add-in icons (`wps-addin/assets/icon-16.png`, `wps-addin/assets/icon-80.png`) and verify `python3 scripts/render_icons.py --check` and `npm test --prefix tests/wps` pass.
- [x] 1.3 Remove `desktop/VALIDATION.md` (keeping `logo/512x512.svg`, `dist/`, `KNOWN_ISSUES.md`, and `tests/TESTING.md`) and verify no code or test files reference `VALIDATION.md`.

## 2. Exclude Font PDFs from Packaged Distributions

- [x] 2.1 Update `scripts/package_native.py` (`populate`) to pass `ignore=shutil.ignore_patterns("*.pdf")` when copying `assets/fonts` to `stage/fonts`, then verify `python3 -m py_compile scripts/package_native.py` passes.
- [x] 2.2 Add a test assertion in `tests/python/test_native_integration.py` confirming staged `fonts/` excludes `*.pdf` while preserving `.ttf` and `OFL.txt` license files, and verify `python3 -m unittest discover -s tests/python -p "test_*.py" -v` passes.

## 3. Rewrite and Reorganize README.md

- [x] 3.1 Rewrite `README.md` into a concise, clearly ordered guide (~250–320 lines, version `0.8.1`) covering overview/highlights, features and ASCII architecture diagram, repository structure, installation & Office integration (Windows/Linux/manual sideloading + troubleshooting/bridge diagnostics), desktop editor quick guide (shortcuts, LaTeX commands, modes, batch), CLI & Rust library usage, building/packaging/CI workflows, brief introductions + links to `KNOWN_ISSUES.md` and `tests/TESTING.md`, and references/credits/license (preserving Tabler Icons `Copyright (c) 2020-2026 Paweł Kuna`, `assets/icons/tabler-icons/LICENSE.txt`, STIX/Nunito/JetBrains Mono OFL notices, `NOTICE`, and `LICENSE` without referencing `desktop/VALIDATION.md`).
- [x] 3.2 Verify all relative links in `README.md` point to existing files and run `./tests/run_all.sh --quick` to confirm workspace Rust, Python, icon, and WPS tests pass.
