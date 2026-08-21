# Factory — Learning Notes

## Mental Model

The Factory pattern separates the decision of *which* concrete type to create from the *use* of that type. Instead of the caller directly instantiating a struct (e.g., `PostgresDb { ... }`), the caller asks a factory for a "Database" and receives a concrete implementation hidden behind a uniform interface.

In Rust, this pattern is driven by the need to decouple systems or swap implementations at runtime (like plugins or configurations). However, because Rust does not have traditional OOP inheritance, Factories usually return `enum`s or trait objects (`Box<dyn Trait>`) rather than base class pointers.

## Structure & Participants

### Product
- **Role:** The interface or unified type that the factory returns.
- **In Rust:** An `enum` (if the variants are known at compile time) or a `Trait` (if open-ended polymorphism is needed).

### Factory
- **Role:** The entity responsible for instantiating the Product.
- **In Rust:** A standalone function, a method on a struct, or a Trait (for Abstract Factories).

## Idiomatic Rust Implementation & Worked Example

Let's look at building a database connection layer.

### Stage 0: Direct Instantiation (Tight Coupling)

```rust
pub struct PostgresDb { pub url: String }
pub struct SqliteDb { pub path: String }

// The caller must know exactly which DB they are using and handle them explicitly.
```

### Stage 1: Factory Method (Enums)

When the set of possible products is closed (known at compile-time), an `enum` is the most idiomatic, performant way to implement a factory in Rust. It avoids heap allocation and dynamic dispatch.

```rust
pub struct PostgresDb { pub url: String }
pub struct SqliteDb { pub path: String }

pub enum DbType {
    Postgres(String),
    Sqlite(String),
}

pub enum Database {
    Postgres(PostgresDb),
    Sqlite(SqliteDb),
}

impl Database {
    pub fn execute(&self, query: &str) {
        match self {
            Database::Postgres(db) => println!("Executing {} on PG at {}", query, db.url),
            Database::Sqlite(db) => println!("Executing {} on SQLite at {}", query, db.path),
        }
    }
}

pub struct DbFactory;

impl DbFactory {
    pub fn connect(db_type: DbType) -> Database {
        match db_type {
            DbType::Postgres(url) => Database::Postgres(PostgresDb { url }),
            DbType::Sqlite(path) => Database::Sqlite(SqliteDb { path }),
        }
    }
}

// Usage
let db = DbFactory::connect(DbType::Sqlite("local.db".to_string()));
db.execute("SELECT * FROM users");
```

### Stage 2: Abstract Factory (Traits with `Send + Sync`)

When you need an open-ended architecture (e.g., loading plugins, or providing a mock implementation for tests), you use Traits and dynamic dispatch. **Crucially**, if these objects will be shared across threads, you must include `Send + Sync` bounds.

```rust
use std::sync::Arc;

pub trait Formatter: Send + Sync {
    fn format(&self, data: &str) -> String;
}

pub trait Parser: Send + Sync {
    fn parse(&self, data: &str) -> String;
}

// The Abstract Factory Trait
pub trait ProtocolFactory: Send + Sync {
    fn create_formatter(&self) -> Box<dyn Formatter>;
    fn create_parser(&self) -> Box<dyn Parser>;
}

// Concrete Implementations
pub struct JsonFormatter;
impl Formatter for JsonFormatter {
    fn format(&self, data: &str) -> String { format!("{{ 'data': '{}' }}", data) }
}

pub struct JsonParser;
impl Parser for JsonParser {
    fn parse(&self, _data: &str) -> String { "parsed".to_string() }
}

pub struct JsonProtocolFactory;
impl ProtocolFactory for JsonProtocolFactory {
    fn create_formatter(&self) -> Box<dyn Formatter> { Box::new(JsonFormatter) }
    fn create_parser(&self) -> Box<dyn Parser> { Box::new(JsonParser) }
}

// Usage
let factory: Arc<dyn ProtocolFactory> = Arc::new(JsonProtocolFactory);
let formatter = factory.create_formatter();
println!("{}", formatter.format("hello"));
```

## When This Pattern Dissolves in Rust

The Factory pattern is often just a simple function in Rust: `fn create_thing() -> Thing`. Creating entire `Factory` structs just to group a single creation method is an anti-pattern imported from Java.

## Versus

- **Builder:** A factory creates a fully formed object in one shot, often hiding the concrete type. A builder constructs a specific type step-by-step.
- **Dependency Injection:** DI often uses factories under the hood to instantiate dependencies before injecting them into a system.

## Pitfalls in Depth

### 1. Java-style Factory Classes
- **What goes wrong:** Creating empty structs like `DatabaseFactory` whose only purpose is to host a `create()` method.
- **Why it happens:** Transliterating object-oriented patterns where everything must be a class.
- **How to handle it, and why that works:** Just use a free-standing function (`pub fn create_database() -> Database`). It's perfectly idiomatic in Rust.
- **Trade-offs of the fix:** Less grouping in the documentation, but significantly less boilerplate.

### 2. Forgetting `Send + Sync` on Trait Objects
- **What goes wrong:** You return `Box<dyn Product>` from a factory, but when the caller tries to pass that product to a spawned thread or store it in an `Arc`, the compiler throws an error.
- **Why it happens:** `dyn Trait` in Rust does not automatically implement `Send` or `Sync`.
- **How to handle it, and why that works:** Explicitly add bounds: `Box<dyn Product + Send + Sync>`. This guarantees thread safety.
- **Trade-offs of the fix:** Concrete types returned by the factory are now strictly forced to be thread-safe, which might require adding `Mutex` or `Arc` internally.

### 3. Excessive Dynamic Dispatch
- **What goes wrong:** Performance drops because every factory product is a `Box<dyn Trait>`, leading to heap allocations and vtable lookups everywhere.
- **Why it happens:** Developers default to `dyn Trait` for polymorphism instead of Enums.
- **How to handle it, and why that works:** Use an `enum` if the number of variants is closed and known at compile time.
- **Trade-offs of the fix:** Enums are sized to their largest variant (which can waste memory if unbalanced) and you cannot add new variants from external crates (closed polymorphism).

## Design Decisions & Trade-offs

- **Enums vs Traits:** Default to Enums for factories. Only use Traits (`Box<dyn Trait>`) if you need open-ended polymorphism (e.g., users of your library providing their own implementations).
- **Associated Types:** In an Abstract Factory trait, you can use associated types instead of `Box<dyn Trait>` to return concrete types and avoid dynamic dispatch, but this makes the factory much more complex to use in collections.

## Exercises & Self-Test

1. **Build Exercise:** Write a `Logger` factory that returns an enum with `StdoutLogger` and `FileLogger` variants.
2. **Build Exercise:** Convert the `Logger` factory to use dynamic dispatch (`Box<dyn Logger + Send + Sync>`) and write a multi-threaded test that logs from multiple spawned threads.
3. Why might you use a free-standing function instead of a `Factory` struct in Rust?

## Open Questions

- How do you ergonomically manage dependencies required by an Abstract Factory (e.g., passing a database connection pool into a factory that creates services)?
- Can associated types completely replace dynamic dispatch in Abstract Factories without ruining ergonomics?

## References

- [Rust Design Patterns - Factory](https://rust-unofficial.github.io/patterns/idioms/pass-and-return.html)
