# Proposal

## Why

WPS already submits up to eight conversions concurrently in ribbon Toggle TeX, but delays all replacements until conversion finishes, retaining all results for the selection in memory and delaying the first visible update. Word VBA runs single-threaded on the host macro thread and keeps its existing Toggle TeX flow outside this change.

## What Changes

- Use a sliding window of up to eight formulas in both directions of ribbon Toggle TeX in WPS.
- Start applying results as soon as the next formula in reverse document order is ready. Keep document writes serial and continue bridge conversion while replacements are in progress.
- Release per-formula conversion data as each item retires and preserve source content for failed items.

## Capabilities

### New Capabilities

### Modified Capabilities

- `office-addin-batch-reliability` gains WPS Toggle TeX scheduling, ordering and buffering requirements.

## Impact

The change covers the WPS ribbon implementation (`wps-addin/main.js`) and its validation (`tests/wps/test_wps_insertion.js`). It reuses the existing snippet conversion endpoint. The Word VBA macro and DOTM remain unchanged.
