# Proposal

## Why

Users can miss WPS Toggle TeX progress because the current implementation changes the Ribbon button label only during MathType insertion. A counter at the lower-left of Writer gives both conversion directions a consistent place to show progress.

## What Changes

- Give the Toggle TeX button a fixed label throughout the operation.
- Show processed and total equation counts at the left edge of a temporary pane docked immediately above the Writer status bar during batch conversion in both directions.
- Clear the EqnTide status when the batch finishes or fails and retain the final conversion summary.

## Capabilities

### New Capabilities

### Modified Capabilities

- `office-addin-batch-reliability` covers WPS lower-left progress in both Toggle TeX directions.

## Impact

The WPS Ribbon definition, batch progress code, regression tests, and desktop Writer verification are affected.
