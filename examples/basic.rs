use dumpit::Dump;

// A type that does NOT implement Debug
struct Opaque {
    _data: Vec<u8>,
}

#[derive(Dump)]
struct Config {
    name: String,
    port: u16,
    secret: Opaque, // no Debug → shows "<!Debug>"
}

fn main() {
    let cfg = Config {
        name: "my-service".to_string(),
        port: 8080,
        secret: Opaque {
            _data: vec![1, 2, 3],
        },
    };
    println!("{:?}", cfg);
    // Config { name: "my-service", port: 8080, secret: <!Debug> }
}
