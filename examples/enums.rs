use dumpit::Dump;

#[allow(dead_code)]
struct Payload(Vec<u8>);

#[derive(Dump)]
enum Message {
    Text(String),
    Binary(Payload),
    Ping,
    Typed {
        kind: String,
        #[dump(skip)]
        _raw: Vec<u8>,
    },
}

fn main() {
    let msgs = vec![
        Message::Text("hello".to_string()),
        Message::Binary(Payload(vec![0xDE, 0xAD])),
        Message::Ping,
        Message::Typed {
            kind: "json".to_string(),
            _raw: vec![1, 2, 3],
        },
    ];

    for msg in &msgs {
        println!("{:?}", msg);
    }
}
