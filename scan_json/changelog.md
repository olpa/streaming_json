## [2.0.0] - 2025-10-xx

- Rewrite for `no_std` support. Incompatible change of the API.


## [1.1.0] - 2025-07-01

- Change interface of the `scan` function
- Implement identity transformation `idtransform` and provide `copy_atom`
- Allow triggering on arrays
- Allow triggering on basic values (strings, numbers, booleans, null)
- Trigger on all objects, not only on unnamed in array context
- An option to stop as soon as possible instead of scanning the whole stream
- Add `IOError` to `ScanError`


## [1.0.2] - 2025-05-03

- Add input stream position to all errors
- Add `MatcherDebugPrinter` to track matching parameters


## [1.0.1] - 2025-02-11

- Update the dependency on `rjiter`


## [1.0.0] - 2025-02-11

- First public release
