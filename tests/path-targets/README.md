# Path target GPU regression

This standalone macOS/Metal harness compiles the production `PathTargets` owner.
It checks texture lifetimes after constructor handles are dropped, attachment
clear/resolve, bind-group sampling, reuse and resize with both 1× and 4× sampling.
Every output pixel is read back and compared with its expected color.

```sh
cargo test --manifest-path tests/path-targets/Cargo.toml --locked
cargo clippy --manifest-path tests/path-targets/Cargo.toml --locked --all-targets -- -D warnings
cargo fmt --manifest-path tests/path-targets/Cargo.toml --check
```

This requires a Metal adapter. GLES correctness was separately checked with
overlapping curved paths on an OHOS phone; see the
[memory measurement and first-use cost](../../docs/path-target-memory-2026-10-13.md).
