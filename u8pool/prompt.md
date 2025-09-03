# U8Pool Implementation Assignment

You are a principal software developer responsible for designing and managing the implementation of the new Rust crate `u8pool`.

## Project Overview

`u8pool` is an allocation-free vector/stack/dictionary for variable-sized slices, designed for performance-critical applications where heap allocation must be avoided.

## Core Specification

### Architecture
- **Buffer Management**: Client pre-allocates a buffer and passes it to the `u8pool` constructor
- **Memory Layout**: Data is stored contiguously in the provided buffer with metadata tracking slice boundaries
- **Zero-Copy Access**: All access methods return references to slices within the buffer

### Primary Interface

#### Core Methods
- `add(slice: &[u8])` - Copies binary data into buffer and creates an indexed slice
- `clear()` - Resets the u8pool to empty state
- `pop()` - Removes the last element from the vector

#### Dictionary Interface
The dictionary functionality uses a key-value pairing convention:
- **Even indices** (0, 2, 4, ...) contain keys
- **Odd indices** (1, 3, 5, ...) contain values
- If element count is odd, the last element is treated as an unpaired key

#### Specialized Dictionary Methods
- `add_key(slice: &[u8])` - Adds a key, replacing if last element is already a key
- `add_value(slice: &[u8])` - Adds a value, replacing if last element is already a value

### Required Features
- **Iterators**: Implement standard Rust iterator patterns for vector, stack, and dictionary access
- **Standard Interface Compatibility**: Adapt common vector/stack/dict methods as appropriate
- **Memory Safety**: Ensure all operations are bounds-checked and safe
- **Performance**: Optimize for minimal overhead and cache efficiency

## Implementation Plan Requirements

Create a detailed implementation plan in `plan.md` following these guidelines:

### Task Structure
- **One task per section** - Each section represents a discrete implementation unit
- **Size limit**: Maximum 100 lines of new functionality per task (excluding tests and documentation)
- **Self-contained**: Each task should be independently implementable

### AI Agent Instructions
For each task, provide:
- **Context**: Sufficient background for the agent to understand the requirements
- **Implementation hints**: Technical guidance and architectural decisions
- **Test-first approach**: Agent must write comprehensive tests before implementation
- **Confirmation requirement**: Agent must seek approval before proceeding with implementation

### Documentation Requirements
Each task must include updates to:
- **Human documentation**: `README.md` and `cargo doc` comments
- **AI documentation**: `doc/llms.txt` and `doc/llms-all.txt`

### Quality Standards
- **Memory safety**: All operations must be bounds-checked
- **Performance**: Minimize allocations and optimize for cache locality
- **Error handling**: Proper error types and handling for buffer overflow conditions
- **Testing**: Comprehensive unit tests including edge cases and error conditions

## Additional Considerations

Please ask for clarification on any aspect of the specification before creating the implementation plan. Consider edge cases, error conditions, and performance implications in your design.
