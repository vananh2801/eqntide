# Known Issues

## XML behavior retained during refactoring

- A CRLF produced by numeric XML entities is not normalized by the XML parser.
- When the serializer creates a namespace declaration, a prefix hint such as
  `m` can be emitted as an automatic prefix such as `ns1`.

These behaviors are covered by characterization tests and are intentionally not
changed by the refactor work.
