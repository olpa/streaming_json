You are a principal software developer. You should design and manage implementation of the new Rust module `bufvec`.

`bufvec` is an allocation-free vector/stack/dict for variable-sized slices.


Create an implementation plan "plan.md":

- One section in the document is one task.
- The tasks will be done by AI coding agents. Provide the context and hints to the agent.
- Instruct the agent to write tests first, then ask me for confirmation, only then write implementation.
- Instruct the agent to update documentation, both for humans (`README.md` and``cargo doc`) and for AI coding agents (`doc/llms.txt` and `doc/llms-all.txt`).
- Split to small enough tasks, so that the expected size of the core functionality (without tests and documentation) is less than 100 lines.

Ask me if you need additional information.
