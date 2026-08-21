# Flyweight — Quick Reference

## One-Liner

Structurally compresses memory and improves CPU cache coherence by extracting shared, invariant data (intrinsic) from millions of objects into a central pool, leaving the objects as lightweight contexts (extrinsic) holding only unique data and an identifier.

## When to Use / When Not

| Use it when | Avoid it when |
| --- | --- |
| You have massive numbers of objects (10,000+) causing memory exhaustion. | You only have a few objects; the pool overhead is unjustified. |
| You need to reduce cache misses by packing data tightly (Data-Oriented Design). | The state is highly unique per instance and cannot be shared. |
| You want to avoid complex lifetime lifetimes (`&'a T`) in your domain structs. | The shared state needs to be mutated dynamically by clients. |

## Structure Sketch

```rust
// Intrinsic (Shared) State
struct MeshData { vertices: Vec<f32> } 

// Factory / Arena
struct MeshArena { 
    meshes: Vec<MeshData> // Contiguous memory for cache coherence
}

// Extrinsic (Context) State
struct Entity {
    x: f32,
    y: f32,
    mesh_id: usize, // Index instead of Rc or &'a
}
```

## Rust Idiom

- **Index-Based (Arena/DOD):** The absolute standard for high-performance Rust. Store heavy state in a `Vec<T>`, store `usize` or `u32` indices in the extrinsic objects. 
- **String Interning:** Use crates like `string_interner` to turn heavy strings into cheap `u32` symbols.
- **`Rc<T>` / `Arc<T>`:** Avoid in hot loops. While ergonomic for general sharing, they scatter memory and cause CPU cache misses when iterating over thousands of flyweights.

## Versus

| Confused with | Key difference |
| --- | --- |
| Singleton | Singleton = exactly one global instance. Flyweight = one instance *per unique state*, many total (e.g. one Oak, one Pine). |
| Prototype | Prototype creates *new copies* of an object. Flyweight *shares the exact same* object across contexts. |

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| `Rc` Cache Thrashing | Use `Vec<T>` arenas and `usize` indices instead of `Rc`. | CPU stalling on cache misses when resolving heap pointers in a loop. |
| Ghost Flyweights | Use generational arenas or explicit ref-counting if dynamic deletion is needed. | The Arena holding onto heavy data forever after all contexts are destroyed. |
| Mutating Intrinsic State | Keep intrinsic state strictly immutable. Move varying data to the context. | Unintentionally changing all entities when trying to alter just one. |
| Inefficient Lookups | Resolve flyweight IDs at asset-load time, not inside hot loops. | Spending more CPU time hashing string keys than you save in memory. |

## Rules of Thumb

- **Intrinsic state:** Heavy, shared, immutable. Lives in the Arena.
- **Extrinsic state:** Lightweight, unique, mutable. Lives in the Client/Entity.
- A `usize` is 8 bytes. If the intrinsic state is smaller than 8 bytes, do not use a Flyweight; just copy the data.
- If you find yourself writing `Rc<RefCell<T>>` in a Flyweight, your separation of intrinsic vs extrinsic state is fundamentally broken.

## Key References

- [Game Programming Patterns: Flyweight](https://gameprogrammingpatterns.com/flyweight.html)
- [Data-Oriented Design](https://www.dataorienteddesign.com/dodmain/)
