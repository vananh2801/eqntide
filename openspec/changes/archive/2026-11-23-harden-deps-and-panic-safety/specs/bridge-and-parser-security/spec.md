# Spec Delta

## ADDED Requirements

### Requirement: Bridge session synchronization recovers from poisoned locks without panicking
The local HTTP bridge session store SHALL tolerate poisoned synchronization locks without panicking or terminating active bridge request-handling threads, returning clean unavailable or error outcomes or recovering consistent session state across session lifecycle operations.

#### Scenario: Poisoned session lock does not crash the HTTP bridge
- **WHEN** a thread panics while holding an internal session synchronization lock and a subsequent client or editor invokes session open, lookup, update, cancel, wait, or stale-cleanup operations
- **THEN** the bridge MUST NOT panic across subsequent session requests and MUST continue serving valid HTTP responses or clean error states

### Requirement: XML and DOCX parsing dependencies comply with workspace security advisories and license policy
Workspace XML and OOXML container dependencies SHALL be free of unresolved security advisories and disallowed licenses under the repository's dependency audit checks while preserving bounded memory and linear-time attribute parsing guarantees.

#### Scenario: Dependency audit passes with adversarial XML and archive bounds intact
- **WHEN** the repository dependency security and license audit is executed and malformed XML start tags or oversized DOCX entries are parsed
- **THEN** the dependency audit MUST pass without unaddressed vulnerability failures and parsers MUST reject malformed or oversized inputs without panicking or quadratic CPU stalling
