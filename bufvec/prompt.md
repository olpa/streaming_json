You are a principal software developer in charge of designing and managing the implementation of the new Rust crate `bufvec`.

`bufvec` is a allocation-free vector/stack/dict for variable-sized slices.

<bufvec summary>

The client pre-allocates a buffer and passes it to a `bufvec` constructor.

The main method is `add`, which takes a slice reference as a parameter. It copies the binary data into the buffer and creates a slice inside it. Future access methods will return references to the slice.

A `bufvec` is append-only, with two exceptions:

- `clear`: resets `bufvec`
- `pop`: removes the last element from the vector

The dictioanry aspect of `bufvec` follows a specific convention: a key is an element with an even index, and the following element is its value. If there are an odd number of elements in `bufvec`, the last element is ignored.

There are unique methods `add_key` and `add_value`. `add_value` acts like `add` if the last element in `bufvec` is a key, and like `replace` if it is a value. Similarly, `add_key` is `add` if the last element is a value, and `replace` if it is a key.

</bufvec summary>

I will need iterators to access the content of `bufvec`. For other access methods similar to the Rust standard vector/stack/dict interfaces, use your common sense to determine if they are needed and how they should be adapted.

Create an implementation plan "plan.md":

- Each section in the document should represent one task.
- Tasks will be assigned to AI coding agents. Provide necessary context and hints to the agent.
- Instruct the agent to write tests first, then seek confirmation from me before proceeding with the implementation.
- The agent should update documentation for both humans (`README.md` and `cargo doc`) and AI coding agents (`doc/llms.txt` and `doc/llms-all.txt`).
- Divide tasks into small enough sections so that the expected size of the new functionality (excluding tests and documentation) is less than 100 lines.

Feel free to ask me if you require any additional information.
