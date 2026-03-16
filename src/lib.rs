//! A derive macro like `#[derive(Debug)]` that doesn't require all fields to implement
//! `Debug`. Non-Debug fields display as `<!Debug>` instead of causing a compile error.
//!
//! # Basic Usage
//!
//! ```
//! use dumpit::Dump;
//!
//! struct Opaque;
//!
//! #[derive(Dump)]
//! struct Config {
//!     name: String,
//!     secret: Opaque,
//! }
//!
//! let cfg = Config { name: "app".into(), secret: Opaque };
//! assert_eq!(format!("{:?}", cfg), r#"Config { name: "app", secret: <!Debug> }"#);
//! ```
//!
//! # Attributes
//!
//! All single-argument attributes support both `Meta::NameValue` (`key = value`) and
//! `Meta::List` (`key(value)`) forms. Multiple arguments require `Meta::List`.
//!
//! ## `#[dump(skip)]`
//!
//! Omit the field from output.
//!
//! ```
//! use dumpit::Dump;
//!
//! #[derive(Dump)]
//! struct User {
//!     name: String,
//!     #[dump(skip)]
//!     internal_id: u64,
//! }
//!
//! let u = User { name: "Alice".into(), internal_id: 42 };
//! assert_eq!(format!("{:?}", u), r#"User { name: "Alice" }"#);
//! ```
//!
//! ## `#[dump(skip_if = "condition")]`
//!
//! Omit the field when the condition evaluates to `true`. The expression has access to `self`.
//!
//! ```
//! use dumpit::Dump;
//!
//! #[derive(Dump)]
//! struct User {
//!     name: String,
//!     #[dump(skip_if = "self.email.is_empty()")]
//!     email: String,
//! }
//!
//! let u1 = User { name: "Alice".into(), email: String::new() };
//! assert_eq!(format!("{:?}", u1), r#"User { name: "Alice" }"#);
//!
//! let u2 = User { name: "Bob".into(), email: "bob@example.com".into() };
//! assert_eq!(format!("{:?}", u2), r#"User { name: "Bob", email: "bob@example.com" }"#);
//! ```
//!
//! ## `#[dump(format("fmt", args...))]`
//!
//! Format the field using a `format!`-style string. Arguments can reference `self`.
//!
//! ```
//! use dumpit::Dump;
//!
//! #[derive(Dump)]
//! struct Person {
//!     #[dump(format("{} years", self.age))]
//!     age: u32,
//! }
//!
//! let p = Person { age: 30 };
//! assert_eq!(format!("{:?}", p), "Person { age: 30 years }");
//! ```
//!
//! ## `#[dump(literal = value)]`
//!
//! Replace the field output with a literal value.
//!
//! ```
//! use dumpit::Dump;
//!
//! #[derive(Dump)]
//! struct Credentials {
//!     user: String,
//!     #[dump(literal = "<redacted>")]
//!     #[allow(dead_code)]
//!     password: String,
//! }
//!
//! let c = Credentials { user: "admin".into(), password: "secret".into() };
//! assert_eq!(format!("{:?}", c), r#"Credentials { user: "admin", password: "<redacted>" }"#);
//! ```
//!
//! ## `#[dump(with = "path::to::function")]`
//!
//! Use a custom function to format the field. The function signature must be
//! `fn(&&FieldType, &mut Formatter) -> fmt::Result`.
//!
//! ```
//! use dumpit::Dump;
//!
//! fn mask(_: &&String, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//!     f.write_str("****")
//! }
//!
//! #[derive(Dump)]
//! struct Config {
//!     #[dump(with = "mask")]
//!     api_key: String,
//! }
//!
//! let c = Config { api_key: "sk-12345".into() };
//! assert_eq!(format!("{:?}", c), "Config { api_key: **** }");
//! ```
//!
//! ## `#[dump(take = N)]`
//!
//! Call `.iter().take(N)` on the field and debug-print the collected elements.
//! The field name becomes `name(n/total)` showing how many elements were taken
//! vs the collection's total length.
//!
//! ```
//! use dumpit::Dump;
//!
//! #[derive(Dump)]
//! struct Data {
//!     #[dump(take = 3)]
//!     items: Vec<i32>,
//! }
//!
//! let d = Data { items: vec![10, 20, 30, 40, 50] };
//! assert_eq!(format!("{:?}", d), "Data { items(3/5): [10, 20, 30] }");
//! ```
//!
//! ## `#[dump(truncate = N)]`
//!
//! Truncate the debug output to at most `N` characters, appending `...` if exceeded.
//!
//! ```
//! use dumpit::Dump;
//!
//! #[derive(Dump)]
//! struct Post {
//!     #[dump(truncate = 10)]
//!     body: String,
//! }
//!
//! let p = Post { body: "Hello, world!".into() };
//! assert_eq!(format!("{:?}", p), r#"Post { body: "Hello, wo... }"#);
//! ```
//!
//! # Generics
//!
//! For concrete types, `Dump` automatically detects whether a field implements `Debug`.
//! For generic type parameters, add a `Debug` bound to get proper output:
//!
//! ```
//! use dumpit::Dump;
//!
//! #[derive(Dump)]
//! struct Wrapper<T: std::fmt::Debug> {
//!     value: T,
//! }
//!
//! let w = Wrapper { value: 42 };
//! assert_eq!(format!("{:?}", w), "Wrapper { value: 42 }");
//! ```
//!
//! Without the bound, generic fields display as `<!Debug>`.
//!
//! # Enums
//!
//! All enum variant types are supported.
//!
//! ```
//! use dumpit::Dump;
//!
//! struct Opaque;
//!
//! #[derive(Dump)]
//! enum Message {
//!     Text(String),
//!     Binary(Opaque),
//!     Ping,
//! }
//!
//! assert_eq!(format!("{:?}", Message::Text("hi".into())), r#"Text("hi")"#);
//! assert_eq!(format!("{:?}", Message::Binary(Opaque)), "Binary(<!Debug>)");
//! assert_eq!(format!("{:?}", Message::Ping), "Ping");
//! ```

pub use dumpit_macros::Dump;

use core::fmt;

// ---------------------------------------------------------------------------
// Autoref specialization for Debug fallback
//
// `DebugWrap(value)` has two `__dumpit_build()` methods:
//
// 1. Inherent on `DebugWrap<&T>` where `T: Debug` — returns a DebugAs that
//    calls `Debug::fmt`.
// 2. Trait `DebugFallbackBuild` on `DebugWrap<T>` for all `T` — returns a
//    DebugAs that prints `<!Debug>`.
//
// When calling `DebugWrap(&field).__dumpit_build()`, method resolution picks
// (1) when T: Debug (inherent takes priority), and (2) otherwise.
// ---------------------------------------------------------------------------

pub struct DebugAs<F: Fn(&mut fmt::Formatter<'_>) -> fmt::Result>(pub F);

impl<F: Fn(&mut fmt::Formatter<'_>) -> fmt::Result> fmt::Debug for DebugAs<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.0)(f)
    }
}

pub struct DebugWrap<T>(pub T);

// Inherent — wins when T: Debug
impl<T: fmt::Debug> DebugWrap<&T> {
    #[inline]
    pub fn __dumpit_build(&self) -> impl fmt::Debug + '_ {
        DebugAs(move |f: &mut fmt::Formatter<'_>| fmt::Debug::fmt(self.0, f))
    }
}

// Trait fallback — loses to inherent
pub trait DebugFallbackBuild {
    fn __dumpit_build(&self) -> impl fmt::Debug;
}

impl<T> DebugFallbackBuild for DebugWrap<T> {
    fn __dumpit_build(&self) -> impl fmt::Debug {
        DebugAs(|f: &mut fmt::Formatter<'_>| f.write_str("<!Debug>"))
    }
}

// ---------------------------------------------------------------------------
// Formatted — wraps a pre-formatted String, displays it verbatim
// ---------------------------------------------------------------------------

pub struct Formatted(pub String);

impl fmt::Debug for Formatted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

// ---------------------------------------------------------------------------
// WithFn — wraps a reference and a formatting function
//   fn(&T, &mut fmt::Formatter) -> fmt::Result
// ---------------------------------------------------------------------------

pub struct WithFn<T, F>(pub T, pub F);

impl<T, F> fmt::Debug for WithFn<T, F>
where
    F: Fn(&T, &mut fmt::Formatter<'_>) -> fmt::Result,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.1)(&self.0, f)
    }
}

// ---------------------------------------------------------------------------
// Truncated — debug-formats the inner value, then truncates to N chars.
// Uses the same autoref specialization pattern for truncation.
// ---------------------------------------------------------------------------

pub struct TruncateWrap<T>(pub T, pub usize);

// Inherent — wins when T: Debug
impl<T: fmt::Debug> TruncateWrap<&T> {
    #[inline]
    pub fn __dumpit_build(&self) -> impl fmt::Debug + '_ {
        let limit = self.1;
        DebugAs(move |f: &mut fmt::Formatter<'_>| {
            let full = format!("{:?}", self.0);
            if full.chars().count() > limit {
                let truncated: String = full.chars().take(limit).collect();
                f.write_str(&truncated)?;
                f.write_str("...")
            } else {
                f.write_str(&full)
            }
        })
    }
}

impl<T> DebugFallbackBuild for TruncateWrap<T> {
    fn __dumpit_build(&self) -> impl fmt::Debug {
        DebugAs(|f: &mut fmt::Formatter<'_>| f.write_str("<!Debug>"))
    }
}

// ---------------------------------------------------------------------------
// TakeIter — calls .iter().take(n) on a collection, formats as
//   (n/total) [elem1, elem2, ...]
// Uses autoref specialization: elements that implement Debug are printed
// normally, otherwise shown as <!Debug>.
// ---------------------------------------------------------------------------

pub struct TakeIter<'a, T>(pub &'a T, pub usize);

impl<'a, T> TakeIter<'a, T>
where
    &'a T: IntoIterator,
    T: TakeIterLen,
{
    /// Returns `"name(n/total)"` for use as the field name.
    pub fn field_name(&self, name: &str) -> String {
        let total = self.0.__dumpit_len();
        let items: Vec<_> = self.0.into_iter().take(self.1).collect();
        let n = items.len();
        format!("{name}({n}/{total})")
    }
}

impl<'a, T> fmt::Debug for TakeIter<'a, T>
where
    &'a T: IntoIterator,
    <&'a T as IntoIterator>::Item: fmt::Debug,
    T: TakeIterLen,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let items: Vec<_> = self.0.into_iter().take(self.1).collect();
        f.debug_list().entries(items.iter()).finish()
    }
}

/// Trait to get the length of a collection. Uses autoref specialization:
/// types with `.len()` use the inherent method; others fall back to
/// `.into_iter().count()`.
pub trait TakeIterLen {
    fn __dumpit_len(&self) -> usize;
}

// Blanket fallback: count via iterator
impl<T> TakeIterLen for T
where
    for<'a> &'a T: IntoIterator,
{
    fn __dumpit_len(&self) -> usize {
        self.into_iter().count()
    }
}
