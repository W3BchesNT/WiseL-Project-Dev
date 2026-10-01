# WiseL-Project-C++

A simple programming language

## Running

```bash
cargo run
```

You can add optional arguments, such as `filename.wise` or `--run`:

```bash
cargo run -- entry.wise
cargo run -- --run
cargo run -- --run input.wise
cargo run -- input.wise --run
```

Adding `--run` will use `clang` and produce a binary for your platform. Without the flag, you get LLVM IR (`out.ll`)
