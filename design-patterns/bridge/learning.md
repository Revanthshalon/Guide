# Bridge — Learning Notes

## Mental Model

The Bridge pattern exists to solve the **Cartesian Product problem**. If an abstraction (what a thing does) and its implementation (how it does it) can both vary independently, classic OOP inheritance forces you to create a subclass for every possible combination (e.g., `LinuxWindow`, `MacWindow`, `LinuxButton`, `MacButton`). 

The Bridge pattern severs this matrix. It separates the abstraction and the implementation into two distinct hierarchies (or trait boundaries) and links them via composition. The core insight: **favor composition over inheritance** to decouple the *interface* the client sees from the *backend* that does the work. You can extend the Abstraction without touching the Implementor, and vice versa.

## Structure & Participants

- **Abstraction:** The high-level policy layer. It defines the interface that clients interact with. It delegates low-level work to the Implementor.
- **Implementor:** The low-level mechanism layer. It provides the primitive operations that the Abstraction uses to build higher-level behavior.
- **Concrete Implementor:** The platform-specific or backend-specific details.

## Idiomatic Rust Implementation

### The TLS Backend Abstraction

A classic systems engineering scenario: you are writing an HTTP client. It needs to establish secure connections, but different environments require different TLS libraries (`rustls` for pure Rust, `native-tls` for OS integration).

```rust
// 1. The Implementor (Low-level primitives)
pub trait TlsBackend {
    fn establish_secure_connection(&self, domain: &str, raw_stream: String) -> Result<String, String>;
}

// 2. Concrete Implementors
pub struct RustlsBackend;
impl TlsBackend for RustlsBackend {
    fn establish_secure_connection(&self, domain: &str, _raw: String) -> Result<String, String> {
        println!("Rustls: Handshaking with {}", domain);
        Ok("rustls_secure_stream".to_string())
    }
}

pub struct OpenSslBackend;
impl TlsBackend for OpenSslBackend {
    fn establish_secure_connection(&self, domain: &str, _raw: String) -> Result<String, String> {
        println!("OpenSSL: Handshaking with {}", domain);
        Ok("openssl_secure_stream".to_string())
    }
}

// 3. The Abstraction (High-level policy)
// We use a generic parameter to bridge the two layers.
pub struct HttpsClient<T: TlsBackend> {
    tls: T,
    max_retries: u32,
}

impl<T: TlsBackend> HttpsClient<T> {
    pub fn new(tls: T, max_retries: u32) -> Self {
        Self { tls, max_retries }
    }

    // High-level operation orchestrating the low-level primitive
    pub fn get(&self, url: &str) -> Result<String, String> {
        let domain = url.split('/').nth(2).unwrap_or("localhost");
        
        for _ in 1..=self.max_retries {
            let raw_tcp = "tcp_stream".to_string(); // Simulated
            if let Ok(stream) = self.tls.establish_secure_connection(domain, raw_tcp) {
                println!("Client: Sending HTTP GET over {}", stream);
                return Ok("HTTP/1.1 200 OK".to_string());
            }
        }
        Err("Failed to connect".to_string())
    }
}

// Usage
fn main() {
    // The Cartesian product is solved. We can mix and match at will.
    let rustls_client = HttpsClient::new(RustlsBackend, 3);
    rustls_client.get("https://example.com/api").unwrap();
}
```

## When This Pattern Dissolves in Rust

As a named pattern, Bridge effectively dissolves in Rust. Rust's core design philosophy (structs for data, traits for behavior, composition instead of inheritance) **is** the Bridge pattern. Because Rust lacks class inheritance entirely, the problem the Bridge originally solved (the explosion of subclasses) cannot literally exist in Rust. 

However, the *mental model*—separating high-level policy structs from low-level mechanism traits—remains deeply relevant when designing library APIs.

## Versus

- **Bridge vs. Adapter:** Adapter makes things work *after* they are designed; it reconciles incompatible interfaces. Bridge is an architectural decision designed *up front* to let abstraction and implementation vary independently. 
- **Bridge vs. Strategy:** Strategy is *behavioral*; it swaps a specific algorithm (e.g., sorting, routing) within a class. Bridge is *structural*; it defines how the entire object is layered, with the Abstraction often having its own complex logic built on top of the Implementor's primitives.

## Pitfalls in Depth

### Pitfall: Bleeding Abstraction (Fat Implementor)

- **What goes wrong:** The `TlsBackend` trait starts accumulating methods that are highly specific to one Concrete Implementor (e.g., `fn set_openssl_cipher_suite()`).
- **Why it happens (the mechanism):** When an advanced feature is needed, the easiest path is to add it to the trait. Eventually, the trait becomes a dumping ground, and all other backends have to provide dummy `unimplemented!()` methods for things they don't support.
- **How to handle it, and why that works:** Keep the Implementor strictly focused on fundamental primitives that apply universally. If an operation fundamentally belongs to only one backend, use backend-specific builder structs to configure the backend *before* passing it into the Bridge abstraction.
- **Trade-offs of the fix:** You cannot configure backend-specific options through the unified Abstraction layer.

### Pitfall: Trait Object Viral Propagation

- **What goes wrong:** Deciding to use `Box<dyn TlsBackend>` to hide generics from the Abstraction's struct signature. It cascades throughout the codebase, forcing allocations and dynamic dispatch everywhere.
- **Why it happens (the mechanism):** Dynamic dispatch (`dyn Trait`) requires a pointer and hides the specific type, which feels cleaner. But if the trait requires `Clone` or returns `Self`, it becomes object-unsafe.
- **How to handle it, and why that works:** Default to generics (`<T: TlsBackend>`) for the Bridge. Only reach for trait objects when you demonstrably need heterogeneous collections (e.g., `Vec<Box<dyn UIControl>>`) or when compile times from monomorphization become a measured problem.
- **Trade-offs of the fix:** Generics infect the caller's signatures, making the types longer (`HttpsClient<RustlsBackend>`).

### Pitfall: The Borrowed Bridge (Lifetime Hell)

- **What goes wrong:** You define the bridge with a reference: `struct HttpsClient<'a> { tls: &'a dyn TlsBackend }`. 
- **Why it happens (the mechanism):** A desire to avoid heap allocation (`Box`) or to share the backend across multiple clients without cloning.
- **How to handle it, and why that works:** A Bridge abstraction should almost always **own** its implementor by value (`<T: Backend>`), or if sharing is required, wrap it in an `Arc<dyn Backend>`. Holding a reference forces a lifetime parameter `'a` onto the client, which infects every struct that holds the client, quickly paralyzing the architecture.
- **Trade-offs of the fix:** `Arc` adds atomic reference counting overhead, but saves you from a borrow checker nightmare.

## Design Decisions & Trade-offs

- **Coarse vs Fine-Grained Implementor:** Should the implementor provide coarse-grained or fine-grained methods? Fine-grained (primitive) methods mean fewer methods to implement for a new backend, but the abstraction does more work. Coarse-grained methods allow backends to optimize, but require more implementation work. In Rust, you can use trait provided methods to get both: require the primitive, provide a coarse-grained default, and let backends override it.

## Exercises & Self-Test

1. Design an Abstraction `MessageBus` (with concrete variants `BufferedBus` and `ImmediateBus`) and an Implementor `PubSubBackend` (with `Kafka` and `RabbitMQ`). Write the Rust trait and struct definitions.
2. What is the fundamental difference in intent between a Bridge and an Adapter, given they look structurally similar?
3. Why is storing a `&'a dyn Backend` inside an Abstraction struct usually a mistake in Rust?
4. How does Rust's lack of inheritance prevent the Cartesian Product problem that Bridge was originally designed to solve?

## Open Questions

- When using trait objects in a Bridge, how do you best handle backends that need fundamentally different initialization configuration before they can be boxed?

## References

- [Async & I/O](../../performance-optimization/async-and-io/learning.md)
- [Zero-Cost Abstractions](../newtype-and-zero-cost/learning.md)
