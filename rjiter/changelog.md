## [1.2.0] - 2025-10-13

- Support `no_std` environment
- Use `embedded-io` instead of `std::io` traits


## [1.1.4] - 2025-05-03

- Fix incorrest detection of an utf8 leading byte
- Fix the range to search for an escape


## [1.1.3] - 2025-03-11

- Fix `InvalidUnicodeCodePoint` when the buffer border breaks a multibyte character


## [1.1.2] - 2025-02-11

- Add missing `std::error::Error` trait implementation
- Mention `scan_json` package in the README
- Satisfy the linter with more rules (`indexing_slicing`, `unwrap_used`)
- Add internal comments about the invariants and contracts


## [1.1.0] - 2025-01-27

- First public release
