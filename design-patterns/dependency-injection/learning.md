# Dependency Injection — Learning Notes

## Mental Model

At its core, Dependency Injection (DI) is a fancy term for passing parameters to a function, but applied to the construction of objects. 

The mental model is: **Never construct a dependency inside the component that uses it; pass it in from the outside.**
Instead of a `CheckoutService` establishing its own database connection, you hand it a database connection pool. This inverses control. The component declares *what* it needs via its constructor signature, and the caller is responsible for fulfilling that contract.

By pushing the instantiation of dependencies all the way up to the application's entry point, you decouple the business logic from infrastructure, making it trivial to substitute components for testing (e.g., swapping a Postgres database for an in-memory mock).

## Structure & Participants

- **The Dependency:** The service, configuration, or connection being injected (e.g., a `UserRepository`).
- **The Client:** The struct that requires the dependency to do its job (e.g., a `CheckoutService`).
- **The Interface (Optional but common):** A trait defining the contract between the Client and the Dependency.
- **The Assembler / Composition Root:** The central location (usually `main.rs`) where the entire application graph is wired together and injected.

## Idiomatic Rust Implementation

In Rust, the most common context for DI is a web server or background worker where state is shared across multiple concurrent request-handling threads. Because of this, dynamic dispatch with `Arc<dyn Trait + Send + Sync>` is the dominant architectural idiom.

```rust
use std::sync::Arc;

// 1. The Interface
// Must be Send + Sync because it will be shared across threads.
pub trait UserRepository: Send + Sync {
    fn get_user(&self, id: u64) -> String;
}

// 2. The Concrete Implementation
pub struct PostgresUserRepository {
    pool: Arc<Pool>, // Dependencies are injected into the constructor
}

impl UserRepository for PostgresUserRepository {
    fn get_user(&self, _id: u64) -> String {
        "User Data".to_string()
    }
}

// 3. The Client (Business Logic)
pub struct CheckoutService {
    users: Arc<dyn UserRepository>,
}

impl CheckoutService {
    // Constructor Injection
    pub fn new(users: Arc<dyn UserRepository>) -> Self {
        Self { users }
    }

    pub fn process(&self, id: u64) {
        let _user = self.users.get_user(id);
    }
}
```
*Note:* We use `Arc<dyn Trait>` rather than `Box<dyn Trait>` because in multi-threaded environments, services are often wrapped in an `Arc` by the web framework to be shared across many worker threads.

## When This Pattern Dissolves in Rust

The *concept* of DI remains vital in Rust, but the *machinery* of DI frameworks dissolves completely.

In Java or C#, DI frameworks reflect over constructors and dynamically resolve dependencies at runtime. Rust's strict type system, trait bounds, and lack of runtime reflection make automatic DI containers both difficult to build and actively harmful to compile times and observability. 

In Rust, the type system *is* the DI framework. **"Pure DI"**—manually wiring up dependencies in `main.rs` by calling constructors and passing them in—is the idiomatic standard. It is type-safe, compile-time verified, and requires no external crates.

## Worked Example

Let's look at a realistic multi-level dependency graph: Configuration → Connection Pool → Repository → Service → Handler.

**Stage 1: The Composition Root (`main.rs`)**
We build the graph from the bottom up, injecting dependencies at each level.

```rust
fn main() {
    // 1. Load config
    let config = Config::load();
    
    // 2. Build infra (leaf nodes)
    let pool = Arc::new(Pool::connect(&config.db_url));
    
    // 3. Build data access layer
    let user_repo = Arc::new(PostgresUserRepository::new(pool.clone()));
    
    // 4. Build domain layer
    let checkout_service = Arc::new(DefaultCheckoutService::new(user_repo));
    
    // 5. Build presentation layer
    let handler = CheckoutHandler::new(checkout_service);
    
    // 6. Start server
    start_server(handler);
}
```

**Stage 2: The Mock Test**
Because `CheckoutService` asks for an `Arc<dyn UserRepository>`, we can easily inject a mock in our unit tests without touching a real database.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    struct MockUserRepository;
    impl UserRepository for MockUserRepository {
        fn get_user(&self, _id: u64) -> String {
            "Mocked User".to_string()
        }
    }

    #[test]
    fn test_checkout() {
        // We inject the mock directly into the service
        let mock_repo = Arc::new(MockUserRepository);
        let service = DefaultCheckoutService::new(mock_repo);
        
        // Assert domain logic...
    }
}
```

## Versus

### Service Locator
- **What's the same:** Both resolve dependencies so a component can do its work.
- **What's different:** A Service Locator hides dependencies. The component reaches into a global registry (`Locator::get::<Database>()`), meaning its signature lies about its requirements. DI forces dependencies to be explicitly declared in the constructor. Service Locators cause runtime crashes; DI causes compile-time errors.
- **How to decide:** Never use Service Locators.

### Factory Pattern
- **What's the same:** Both involve creating objects.
- **What's different:** A Factory is used when you need to create *multiple instances* of an object on-demand at runtime. DI is used to inject *long-lived, shared services* exactly once during application startup.

## Pitfalls in Depth

### Pitfall: DI Container Addiction

- **What goes wrong:** Teams coming from Spring Boot introduce complex Rust crates (like `shaku` or `waiter`) to magically inject dependencies via procedural macros. The code becomes hard to read, IDE jump-to-definition breaks, and compile times soar.
- **Why it happens (the mechanism):** Attempting to force the paradigms of reflection-heavy languages into a statically-bound systems language.
- **How to handle it, and why that works:** Embrace "Pure DI". Write a single `fn assemble_app()` in `main.rs` that explicitly instantiates your database, mailer, and services. If the setup function gets too long, break it into smaller functions (e.g., `init_repositories()`). Explicit wiring takes seconds to compile and is universally readable.
- **Trade-offs of the fix:** Adding a new dependency deep in the graph requires threading it through a few layers of constructors manually.

### Pitfall: Constructor Parameter Explosion

- **What goes wrong:** A service requires 12 different dependencies, resulting in a `fn new(a, b, c, d... )` that is tedious to call and maintain.
- **Why it happens (the mechanism):** The service is violating the Single Responsibility Principle. If it needs 12 dependencies, it is doing too much.
- **How to handle it, and why that works:** Refactor the service into smaller, cohesive domain components. Do *not* bundle the dependencies into a `SystemContext` struct that you pass everywhere.
- **Trade-offs of the fix:** Requires actively rethinking domain boundaries rather than just papering over the problem.

### Pitfall: The `SystemContext` Anti-Pattern

- **What goes wrong:** To avoid passing many parameters, you create a `struct Context { db: Pool, mailer: Mailer, logger: Logger }` and pass `ctx` to every function in the application.
- **Why it happens (the mechanism):** It feels like a convenient way to clean up signatures. But this is just a Service Locator disguised as a struct!
- **How to handle it, and why that works:** Pass explicit dependencies. If a function only needs the database, it should only take the database. If it takes the entire `Context`, it is impossible to test it without mocking the Mailer and Logger as well, making tests brittle and setup heavy.
- **Trade-offs of the fix:** Marginally longer function signatures in exchange for dramatic improvements in testability and component isolation.

### Pitfall: Over-Genericizing Everything

- **What goes wrong:** You define traits for *every single struct* in your application, leading to a sea of `Arc<dyn ...>` or `<T: Config, M: Mailer>` bounds that infect the codebase.
- **Why it happens (the mechanism):** Treating DI as a rigid rule rather than a tool. 
- **How to handle it, and why that works:** Only use interfaces (traits) for boundaries that cross architectural layers (databases, external APIs) or things that *must* be mocked for testing. Pure business logic structs should just take concrete data types.
- **Trade-offs of the fix:** If you later realize you need to mock a component, you have to extract a trait for it after the fact.

## Design Decisions & Trade-offs

**Generics (`impl Trait`) vs. Trait Objects (`Arc<dyn Trait>`):**
- **Generics** offer static dispatch (zero runtime cost) and allow the compiler to inline code. However, they cause monomorphization bloat, force you to infect the struct definition with `<T>`, and can result in verbose trait bounds propagating through your codebase.
- **Trait Objects** require a pointer indirection (`Arc`) and dynamic dispatch (vtable lookup). The performance cost is usually negligible for I/O bound dependencies (like databases). They vastly simplify struct definitions by hiding the generic type.
- **Rule of Thumb:** Use generics for highly reusable libraries or performance-critical tight loops (like parsers). Use `Arc<dyn Trait>` for application-level architectural boundaries (like injecting a database into a Web Handler).

## Exercises & Self-Test

1. Define a `NotificationService` that takes a `dyn SmsClient` and a `dyn EmailClient`. Write the `main` function to assemble it.
2. Why does wrapping a dependency in `Arc<dyn Trait>` require the `Trait` to have `Send + Sync` bounds in a web application context?
3. Explain why passing a `GlobalContext` struct into a service constructor is an anti-pattern. How does it negatively impact unit testing?
4. Write a mock implementation of `UserRepository` that returns predetermined test data without hitting a database.

## Open Questions

- What is the most idiomatic way to handle deeply nested nested dependency graphs when doing Pure DI, without constructors becoming overwhelming?
- How does the `cake pattern` in Scala compare to DI in Rust using traits?

## References

- Mark Seemann, *Dependency Injection Principles, Practices, and Patterns* — The definitive book on the topic.
- Cross-ref: [Repository & Unit of Work](../repository-and-unit-of-work/learning.md) (the most common things you will inject).
