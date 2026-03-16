use dumpit::Dump;

struct NoDebug;

// Generic struct — no Debug bound on T.
// Since Dump uses autoref specialization, generic type params
// without a Debug bound will show as <!Debug>.
// Add `T: Debug` bound if you want generic fields to print.
#[derive(Dump)]
struct Wrapper<T> {
    label: String,
    value: T,
}

// With a Debug bound, generic fields print normally.
#[derive(Dump)]
struct Container<T: std::fmt::Debug> {
    label: String,
    value: T,
}

fn main() {
    // Concrete types always work:
    #[derive(Dump)]
    struct Point {
        x: f64,
        y: f64,
    }
    println!("{:?}", Point { x: 1.0, y: 2.0 });

    // Generic without Debug bound — fields are <!Debug>
    let w = Wrapper {
        label: "test".to_string(),
        value: NoDebug,
    };
    println!("{:?}", w);

    // Generic with Debug bound — fields print normally
    let c = Container {
        label: "count".to_string(),
        value: 42,
    };
    println!("{:?}", c);
}
