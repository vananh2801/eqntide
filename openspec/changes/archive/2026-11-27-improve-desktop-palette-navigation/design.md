# Design

## Context

`desktop/src/app/palette_ui.rs` uses an egui horizontal scroll area around a row-first grid. It sizes the panel from `PALETTE_HEIGHT` plus the scrollbar allocation when needed and keys the scroll area by category. Tile activation currently inserts a template and restores editor focus, or opens the custom matrix dialog.

## Goals / Non-Goals

Keep the existing tile sizing, shortcuts, tooltips, disabled states, and editor focus behavior. Order related variants together in the desktop palette and keep Recent in most-recently-used order. Scope scroll input and chrome changes to the desktop palette.

## Decisions

- Build each column from up to three consecutive tiles in desktop display order. Use catalog identifiers for display ordering and preserve the catalog as the source of item content. Include the custom matrix button in the same layout and width calculation. Use the full ribbon width as the tile viewport and compute overflow from that width.
- Use egui's horizontal clipping and wheel handling. Hide the scroll track and share a single category-scoped offset between wheel and drag input. Clamp that offset against the current tile viewport and content widths on resize or category changes.
- Resolve pointer gestures before running tile actions. Use egui's drag threshold where it can distinguish horizontal intent. A recognized drag owns the rest of the press even if the pointer returns to its origin, reaches a scroll limit, or leaves the viewport. Check whether the existing scroll area can capture a drag that starts on a clickable tile. If it cannot, give the tile viewport and its tiles compatible click and drag handling so they share gesture ownership.
- Apply the palette's disabled state to both navigation inputs and tile actions.

## Risks / Trade-offs

- **A drag can begin over a tile that inserts a template or opens a dialog.** Tests should move the pointer beyond the threshold, release over the original tile, and check that the corresponding action is suppressed. Also check ordinary clicks and short movements.
- **Current layout tests expect the palette scrollbar.** Replace those assertions with checks for fixed height, absence of palette scroll chrome, and reachability of the last column. Keep the other scroll-area assertions as regression coverage.
