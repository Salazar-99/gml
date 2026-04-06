# Installation

Build and install both binaries from the repository:

```bash
cargo install --path crates/gml-cli/cli --locked
cargo install --path crates/gml-cli/daemon --locked
```

Or build from source and run the release artifacts:

```bash
cargo build -p gml -p gml-daemon --release
```

The binaries are `target/release/gml` and `target/release/gmld`.
