# dumpit

A derive macro like `#[derive(Debug)]` that doesn't require all fields to implement `Debug`.
Non-Debug fields display as `<!Debug>`. Field-level attributes control formatting.

See [crate documentation](https://docs.rs/dumpit) for full usage and examples.

```rust
use dumpit::Dump;

struct Opaque;

#[derive(Dump)]
struct Config {
    name: String,
    _internal: Opaque,
    #[dump(literal = "[-- redacted --]")]
    password: String,
    #[dump(truncate = 10)]
    long_text: String,
    #[dump(take = 5)]
    binary: Vec<u8>,
}

let cfg = Config { 
    name: "app".to_string(),
    _internal: Opaque,
    password: "supersecret".to_string(),
    long_text: "Lorem ipsum dolor sit amet".to_string(),
    binary: (0..100).collect(),
};

println!("{:#?}", cfg);

// Outputs:
// r#"Config {
//     name: "app",
//     _internal: <!Debug>,
//     password: "[-- redacted --]",
//     long_text: "Lorem ipsu...",
//     binary(5/100): [0, 1, 2, 3, 4],
// }"#
)

```

## License

Licensed under either of

- [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
- [MIT License](http://opensource.org/licenses/MIT)

at your option.
