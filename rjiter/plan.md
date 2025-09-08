# Migration Plan: RJiter Error Handling to `thiserror`

## Current State Analysis

### Existing Error Structure
- **Location**: `src/error.rs`
- **Main types**:
  - `Error` struct with `error_type: ErrorType` and `index: usize`
  - `ErrorType` enum with variants: `JsonError`, `WrongType`, `IoError`
  - Custom `Display` and `std::error::Error` implementations
- **Dependencies**: Currently uses `std` for error traits and I/O operations
- **Usage**: Extensive error creation through static methods like `from_jiter_error`, `from_io_error`, `from_json_error`

### Existing `thiserror` Experience
The codebase has existing experience with `thiserror` implementation in the `bufvec` crate:
- **Commit a70545a**: Successfully implemented `thiserror` with `no_std` compatibility
- **Pattern used**: Enhanced error enum with `#[error]` attributes while maintaining structured error data
- **Approach**: Improved error ergonomics while preserving all existing error information

## Migration Plan

### Phase 1: Dependency Setup

1. **Update Cargo.toml**
   - Add `thiserror` dependency (standard version since RJiter uses `std`)

   ```toml
   [dependencies]
   jiter = "0.8.2"
   thiserror = "2.0"
   ```

2. **No crate-level changes needed**
   - RJiter remains a `std` crate
   - Continue using `std` types as needed

### Phase 2: Error Type Migration

1. **Transform `ErrorType` enum to use `thiserror`**
   - Add `#[derive(Error, Debug)]` to `ErrorType`
   - Add descriptive `#[error]` attributes with context variables
   - Use `#[from]` for `std::io::Error` since we only have one I/O error variant
   - Maintain existing structured data for `WrongType` variant

   ```rust
   #[derive(Error, Debug)]
   pub enum ErrorType {
       #[error("JSON parsing error: {0}")]
       JsonError(JsonErrorType),
       #[error("Type mismatch: expected {expected}, found {actual}")]
       WrongType {
           expected: JsonType,
           actual: JsonType,
       },
       #[error("I/O operation failed")]
       IoError(#[from] std::io::Error),
   }
   ```

2. **Update `Error` struct approach**
   - Keep the current structure with `error_type` and `index`
   - **Key insight from research**: Use manual `Display` implementation to include index
   - Don't derive `Error` on the main `Error` struct to maintain custom formatting
   - Use `#[source]` to chain the `ErrorType` for proper error causality

   ```rust
   #[derive(Debug)]
   pub struct Error {
       #[source]
       pub error_type: ErrorType,
       pub index: usize,
   }
   
   impl std::error::Error for Error {
       fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
           Some(&self.error_type)
       }
   }
   
   impl std::fmt::Display for Error {
       fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
           write!(f, "{} at index {}", self.error_type, self.index)
       }
   }
   ```

3. **Leverage research findings**
   - Since we have only one `IoError` variant, we can use `#[from]` safely
   - Focus on meaningful error messages with structured context
   - Preserve the existing index-based error location information

### Phase 3: Implementation Details

1. **Error creation methods**
   - Update `from_jiter_error`, `from_json_error` methods to work with new `ErrorType`
   - **Simplify `from_io_error`**: With `#[from]` on `IoError`, this becomes automatic
   - Maintain backward compatibility in method signatures
   - Consider adding helper methods based on research best practices

2. **Display formatting strategy**
   - Let `thiserror` handle `ErrorType` display formatting automatically
   - Keep manual `Display` implementation on main `Error` struct for index information
   - This hybrid approach gives us both structured error messages and positional context
   - Maintain existing `description()` and `get_position()` methods

3. **Error context and chaining**
   - Use `#[source]` attribute to maintain proper error chains
   - Keep all existing error information (JSON types, I/O errors, positions)
   - Benefit from automatic error source chaining via `std::error::Error::source()`
   - Maintain debuggability with structured error data and better formatting

### Phase 4: I/O Error Handling Strategy

**Optimized approach based on research**: Leverage `#[from]` for seamless I/O error handling:

- Keep `IoError(std::io::Error)` variant but add `#[from]` attribute
- This enables automatic conversion via `?` operator throughout the codebase
- Simplify error handling code by removing manual `from_io_error` calls where possible
- `thiserror` handles both `Display` and `From` implementations automatically
- Maintain full `std::io::Error` information since we're a `std` crate
- **Key benefit**: Reduced boilerplate while preserving all error context

### Phase 5: Testing and Validation

1. **Error message validation**
   - Verify improved error messages from `thiserror` formatting
   - Ensure position information ("at index N") is correctly included
   - Test error chain traversal with `std::error::Error::source()`
   - Check that all existing error scenarios produce meaningful messages

2. **Integration testing**
   - Run existing test suite to ensure no regressions
   - Test automatic `io::Error` conversion via `?` operator
   - Verify error conversion methods work correctly
   - Test error Display formatting matches expectations

3. **API compatibility testing**
   - Ensure all public error methods remain unchanged
   - Verify `description()` and `get_position()` methods work as before
   - Test that existing user code continues to compile
   - Validate that error matching/handling patterns still work

4. **Error ergonomics validation**
   - Confirm improved developer experience with better error messages
   - Test that error debugging is enhanced with structured information
   - Verify error source chaining provides useful troubleshooting info

### Phase 6: Documentation and Examples

1. **Update documentation**
   - Document improved error handling with `thiserror`
   - Provide examples of better error messages and handling
   - Update any error-related documentation

2. **Migration guide**
   - Document the upgrade (should be seamless for users)
   - Highlight improved error messages and ergonomics
   - Provide examples of enhanced error handling

## Benefits of This Migration

1. **Better error ergonomics**: Improved error messages and handling via `thiserror`
2. **Reduced boilerplate**: Less manual `Display` and trait implementations
3. **Maintained functionality**: All existing error information and methods preserved
4. **Enhanced developer experience**: Better error messages with structured formatting
5. **Industry standard**: Using well-established `thiserror` crate patterns

## Risks and Mitigation

1. **Breaking changes**: Minimal, as error structure and methods remain similar
2. **New dependency**: `thiserror` is well-established and widely used
3. **Performance**: No significant impact expected, possibly improved due to optimized implementations

## Timeline Estimate (Updated)

- **Phase 1-2**: 1-2 hours (dependency setup and `thiserror` migration)
- **Phase 3-4**: 2-3 hours (implementation refinement and I/O optimization)  
- **Phase 5-6**: 2-3 hours (comprehensive testing and documentation)
- **Total**: 5-8 hours

**Key efficiency gains from research**:
- Using `#[from]` reduces manual conversion code
- Hybrid approach (auto + manual Display) minimizes changes
- Focused testing on error ergonomics improvements

This migration follows the successful pattern established in the `bufvec` crate and should provide improved error ergonomics and developer experience while maintaining full compatibility with existing code.

## `thiserror` Best Practices Research Summary

Based on comprehensive research of current `thiserror` best practices, here are the key findings for replacing `std::io::Error`:

### Core Design Principles

#### When to Use `thiserror`
- **Library code**: Use for library APIs where callers need structured error information
- **Structured errors**: When callers need to handle different error cases programmatically
- **Error type ownership**: When you need domain-specific error types rather than opaque errors

#### Basic Integration Patterns

**Simple Wrapping with `#[from]`:**
```rust
#[derive(Error, Debug)]
pub enum MyError {
    #[error("I/O operation failed")]
    Io(#[from] std::io::Error),
}
```

**Context-Rich Error Handling:**
```rust
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file '{path}': {source}")]
    ReadError {
        path: String,
        #[source]
        source: std::io::Error,
    },
}
```

### Critical Limitations and Solutions

#### The "One From Per Type" Constraint
**Problem**: Cannot have multiple `#[from]` variants for the same source type:
```rust
// This WON'T COMPILE
#[derive(Error, Debug)]
pub enum MyError {
    #[error("Read failed")]
    ReadError(#[from] std::io::Error),  // ❌
    #[error("Write failed")]
    WriteError(#[from] std::io::Error), // ❌ Duplicate From implementation
}
```

**Solution**: Use `#[source]` with manual conversion methods:
```rust
#[derive(Error, Debug)]
pub enum MyError {
    #[error("Read operation failed")]
    ReadError(#[source] std::io::Error),
    #[error("Write operation failed")]
    WriteError(#[source] std::io::Error),
}

impl MyError {
    pub fn read_error(err: std::io::Error) -> Self {
        Self::ReadError(err)
    }
    
    pub fn write_error(err: std::io::Error) -> Self {
        Self::WriteError(err)
    }
}
```

### Context Preservation Strategies

1. **Struct-Based Context**: Include relevant metadata in error variants
2. **Dynamic Error Messages**: Use format strings with context variables
3. **Error Conversion Helpers**: Provide convenient constructors for complex errors
4. **Error Chain Preservation**: Always use `#[source]` to maintain error causality

### Recommended Pattern for RJiter

Based on the research and RJiter's current structure:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ErrorType {
    #[error("JSON parsing error: {0}")]
    JsonError(JsonErrorType),
    
    #[error("Type mismatch: expected {expected}, found {actual}")]
    WrongType {
        expected: JsonType,
        actual: JsonType,
    },
    
    #[error("I/O operation failed")]
    IoError(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub struct Error {
    #[source]
    pub error_type: ErrorType,
    pub index: usize,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at index {}", self.error_type, self.index)
    }
}
```

### Key Research Insights

1. **Library vs Application**: `thiserror` is primarily for libraries, `anyhow` for applications
2. **Context Limitations**: Industry feedback indicates context can be "blurred in type" with `thiserror`
3. **Alternative Considerations**: For complex projects, `snafu` offers more flexibility for context preservation
4. **Best Practice**: Focus on meaningful error messages with structured data rather than just wrapping errors

This research confirms that the planned migration approach is sound, while highlighting the importance of careful error variant design to maximize the benefits of `thiserror`'s structured error handling capabilities.