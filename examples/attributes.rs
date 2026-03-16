use dumpit::Dump;

// A type that does NOT implement Debug
#[allow(dead_code)]
struct Secret(String);

fn format_password(_: &&Secret, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str("[--dedacted--]")
}

#[derive(Dump)]
struct User {
    name: String,
    #[dump(skip)]
    _internal_id: u64,
    #[dump(literal = "<hidden>")]
    #[allow(dead_code)]
    password: Secret,
    #[dump(with = "format_password")]
    api_key: Secret,
    #[dump(format("{} years", self.age))]
    age: u32,
    #[dump(truncate = 20)]
    bio: String,
    #[dump(skip_if = "self.email.is_empty()")]
    email: String,
    #[dump(take = 5)]
    long_vec: Vec<u8>,
}

fn main() {
    let user = User {
        name: "Alice".to_string(),
        _internal_id: 42,
        password: Secret("password".to_string()),
        api_key: Secret("sk-12345".to_string()),
        age: 30,
        bio: "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".to_string(),
        email: String::new(),
        long_vec: (0..100).collect(),
    };
    println!("{:#?}", user);

    println!();

    let user2 = User {
        name: "Bob".to_string(),
        _internal_id: 99,
        password: Secret("pneumonoultramicroscopicsilicovolcanoconiosis".to_string()),
        api_key: Secret("sk-99999".to_string()),
        age: 25,
        bio: "Short bio".to_string(),
        email: "bob@example.com".to_string(),
        long_vec: (0..2).collect(),
    };
    println!("{:#?}", user2);
}
