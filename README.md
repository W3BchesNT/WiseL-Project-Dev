# WiseL-Project-C++

A simple programming language

## Running

```bash
cargo run
```

You can add optional arguments, such as `filename.wise`, `--run`, or `--llvm`:

```bash
cargo run -- entry.wise
cargo run -- --run --llvm input.wise
cargo run -- --run input.wise
cargo run -- input.wise --run
```

Adding `--llvm` will use `inkwell` and `clang` and produce a binary for your platform.
Without the `--run` flag, you get LLVM IR (`out.ll`)
