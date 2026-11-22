# Tasks

## 1. Verify the Writer progress surface

- [x] 1.1 Verify on desktop Writer that the approved bottom-docked pane displays its message at the lower-left, updates while the add-in yields, and closes cleanly. Record the tested build.

## 2. Move Toggle TeX progress

- [x] 2.1 Give Toggle TeX a fixed Ribbon label and remove progress-driven button invalidation. Verify the Ribbon regression tests pass.
- [x] 2.2 Share throttled lower-left pane progress across bridge conversion and document replacement in both directions. Extend the WPS batch tests to verify phase counts, cleanup on completion and failure, and preservation of concurrency and final summary counts.

## 3. Verify in desktop Writer

- [x] 3.1 Run multi-formula Toggle TeX batches in both directions on the verified Writer builds. Confirm that progress is visible at the lower-left during both phases, the button label stays fixed, the status clears, and summary counts match the document.

## Desktop probe result

WPS Writer for Linux 12.1.2.29182 returned `null` from `Application.StatusBar` before and after writing two successive messages. Screenshots showed the regular page and word counts with no probe message. `Document.CommandBars.Item("Status Bar")` returned the visible bar, but adding a temporary control returned `null`. The lower-left progress display remains unverified on this build.

A second desktop probe tested the application-level and document-level status bar collections. Adding a button or label to the native bar returned `null`, including with a documented built-in control ID. The existing control kept its original caption after a temporary assignment. A separate temporary bottom command bar accepted a caption, but screenshots showed no progress text at the lower-left. Test controls and installed add-in files were restored after the probes.

The approved bottom-docked pane was verified on Writer for Linux 12.1.2.29182 with a disposable document. Both Toggle TeX directions reported 480 found, 480 converted, and zero skipped or failed. The document contained 480 inline objects after insertion and zero after replacement with LaTeX. Screenshots captured advancing phase counts on the lower left and the normal viewport after each batch. The regression suite also covers progress cleanup when conversion or insertion fails.
