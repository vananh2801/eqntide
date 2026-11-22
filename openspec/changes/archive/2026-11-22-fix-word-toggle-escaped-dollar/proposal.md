# Proposal

## Why

Word Toggle LaTeX can mistake an escaped dollar for the opening delimiter and consume the opening delimiter of a real formula later in the selection. WPS already checks for escaped opening dollars.

## What Changes

- Preserve literal dollars and recognize the surrounding LaTeX formulas in Word, for selections and at the caret.
- Rebuild the distributed Word template and verify the escaped-dollar cases.

## Capabilities

### New Capabilities

### Modified Capabilities

- `office-addin-batch-reliability` covers escaped dollar recognition in Word Toggle LaTeX.

## Impact

Word VBA source, packaged Word template, and Word Toggle regression coverage.
