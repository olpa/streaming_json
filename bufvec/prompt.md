You are a principal software developer. You should design and manage implementation of the new Rust crate `bufvec`.

`bufvec` is an allocation-free vector/stack/dict for variable-sized slices.

<bufvec summary>
The client pre-allocates a buffer and passes it to a `bufvec` constructor.

The core method is `add` with a slice reference as a parameter. It copies the binary data into the buffer and creates a slice inside the buffer. The future access method will return references to the slice.

A `bufvec` is append-only, except of two methods:

- `clear`: reset `bufvec`
- `pop`: pops the last element from the vector

The dictionary facet of `bufvec` is based on a convention: a key is an element with an even index, and the following element is its value. If there are odd number of elements in `bufvec`, the last element is ignored.

There are pecular methods `add_key` and `add_value`. `add_value` works as `add` if the last element in `bufvec` is a key, and as `replace` if is a value. In a similar way, `add_key` is `add` if the last element is a value, and is `replace` if is a key.
</bufvec summary>

I'll need iterators to access the content of `bufvec`. For other access methods similar to the Rust standard vector/stack/dict interfaces, use your common sense to decide if they are needed and how they should be adapted.

Create an implementation plan "plan.md":

- One section in the document is one task.
- The tasks will be done by AI coding agents. Provide the context and hints to the agent.
- Instruct the agent to write tests first, then ask me for confirmation, only then write implementation.
- Instruct the agent to update documentation, both for humans (`README.md` and``cargo doc`) and for AI coding agents (`doc/llms.txt` and `doc/llms-all.txt`).
- Split to small enough tasks, so that the expected size of the core functionality (without tests and documentation) is less than 100 lines.

Ask me if you need additional information.
