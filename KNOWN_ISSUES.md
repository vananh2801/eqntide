# Known Issues

## XML behavior retained intentionally

- A CRLF produced by numeric XML entities (`&#13;&#10;`) is not normalized to
  LF by the XML parser, matching the W3C XML 1.0 rule for character references.
