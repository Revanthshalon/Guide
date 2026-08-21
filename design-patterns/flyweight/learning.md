# Flyweight — Learning Notes

## Mental Model

Flyweight solves the collision between scale and memory limits by structural compression. When you need millions of objects that share significant overlapping data, you divide the data into two halves:
- **Intrinsic state:** Heavy, invariant, shared (e.g., a 3D mesh, a font glyph, a texture).
- **Extrinsic state:** Lightweight, variant, per-instance (e.g., X/Y coordinates, velocity).

Instead of each object owning both halves, the heavy intrinsic state is extracted, deduplicated, and stored centrally. The millions of instances become lightweight structs containing only their extrinsic state and a pointer (or index) to their shared intrinsic state.

In Rust, this pattern naturally aligns with Data-Oriented Design (DOD). Rather than focusing purely on memory reduction, we use Flyweight to optimize memory *layout*, ensuring cache coherence and predictable memory strides by packing shared state into contiguous arrays (Arenas).

## Structure & Participants

- **Intrinsic State (Flyweight):** The heavy, shared data. Strictly immutable after creation.
- **Arena / Pool / Factory:** Central storage for the intrinsic state. Deduplicates instances and returns an identifier.
- **Extrinsic State (Context):** The lightweight, per-instance data. Holds an identifier pointing into the Arena.
- **Identifier:** How the extrinsic state locates the intrinsic state. In Rust, this should almost always be a `usize` index or a typed ID (newtype), not an `Rc`.

## Idiomatic Rust Implementation

The idiomatic Rust Flyweight avoids `Rc<T>` entirely. `Rc` scatters allocations across the heap, destroying cache locality. Instead, we use **Index-Based Flyweights** (Arenas) coupled with Data-Oriented Design principles.

By storing the heavy shared data in a contiguous `Vec`, we guarantee a predictable memory stride and excellent cache coherence. The extrinsic contexts just hold `usize` indices.

```rust
// 1. Intrinsic State: Heavy and Shared
pub struct MeshData {
    pub vertices: Vec<f32>,
    pub texture_id: u32,
    // Megabytes of data...
}

// 2. The Arena (Factory/Pool)
pub struct MeshArena {
    meshes: Vec<MeshData>,
}

impl MeshArena {
    pub fn new() -> Self {
        Self { meshes: Vec::new() }
    }

    // Deduplication logic would go here. For simplicity, we just push.
    pub fn register(&mut self, mesh: MeshData) -> usize {
        let id = self.meshes.len();
        self.meshes.push(mesh);
        id
    }

    pub fn get(&self, id: usize) -> Option<&MeshData> {
        self.meshes.get(id)
    }
}

// 3. Extrinsic State (Context): Lightweight, lives in thousands of instances
pub struct TreeEntity {
    pub x: f32,
    pub y: f32,
    pub health: f32,
    pub mesh_id: usize, // The Flyweight link
}

fn render_forest(arena: &MeshArena, forest: &[TreeEntity]) {
    for tree in forest {
        // Fetch intrinsic state via index
        let mesh = arena.get(tree.mesh_id).expect("Invalid mesh ID");
        // Compute operation using both states
        println!("Rendering mesh {} at ({}, {})", mesh.texture_id, tree.x, tree.y);
    }
}
```
This is how Entity Component Systems (ECS) in game engines operate. The `usize` index takes 8 bytes, requires no reference counting atomic operations, and trivially sidesteps lifetime/borrowing complexities.

## When This Pattern Dissolves in Rust

Rust's core types naturally enforce Flyweight sharing:
- **`&str` vs `String`:** A `&str` is a flyweight pointer to a shared allocation (e.g., the `.rodata` segment).
- **String Interning:** Crates like `string_cache` turn heavy strings into lightweight `u32` symbols.
- **Enums for State:** Java often uses Flyweight to cache state-machine objects. Rust uses zero-allocation `enum` variants.

## Worked Example

Imagine building a particle system.

**Stage 0: Uncompressed Ownership**
```rust
struct Particle {
    x: f32,
    color: [u8; 4],
    texture: Vec<u8>, // 1MB per particle!
}
// 10,000 particles = 10GB of RAM. Program crashes.
```

**Stage 1: Pointers (`&'a T`)**
```rust
struct Particle<'a> {
    x: f32,
    texture: &'a Vec<u8>, // Shared!
}
// Works, but now your Particle struct is infected with lifetimes.
// Storing Particles in arbitrary data structures becomes a borrow-checker nightmare.
```

**Stage 2: The Arena DOD Flyweight**
```rust
struct Particle {
    x: f32,
    texture_id: u32,
}
// No lifetimes. 8 bytes per particle. 10,000 particles = 80KB.
// The textures live in a single contiguous `Vec<Texture>` in the engine.
```

**Scoreboard:**
- Stage 0: Out of Memory.
- Stage 1: Developer Out of Patience (lifetime hell).
- Stage 2: Blazing fast, cache-friendly, trivial to serialize.

## Versus

### Flyweight vs. Singleton
- **Same:** Both limit allocations.
- **Different:** Singleton enforces exactly *one global instance*. Flyweight enforces exactly *one instance per unique state*, but you can have many different flyweights (e.g., one Oak flyweight, one Pine flyweight).

### Flyweight vs. Prototype
- **Same:** Both optimize instantiation.
- **Different:** Prototype *clones* objects to create new distinct instances. Flyweight *shares* the exact same instance across contexts.

## Pitfalls in Depth

### Pitfall: `Rc`/`Arc` Cache Thrashing

- **What goes wrong:** You use `Rc<HeavyState>` in your extrinsic structs. Memory usage drops, but CPU utilization skyrockets, and framerates tank.
- **Why it happens (the mechanism):** `Rc` allocates on the heap. When you loop over 10,000 entities, resolving the `Rc` pointer forces the CPU to jump across random memory addresses. This causes constant CPU cache misses. Furthermore, `Rc` cloning and dropping incur reference-count branching overhead.
- **How to handle it, and why that works:** Use an Arena (`Vec<HeavyState>`) and `usize` indices. The `Vec` guarantees a contiguous memory stride. The hardware prefetcher can easily predict and load the data into the L1/L2 cache before the CPU needs it.
- **Trade-offs of the fix:** You lose automatic garbage collection. If a shared state is no longer referenced by any context, it won't be automatically dropped until the Arena is cleared.

### Pitfall: Inefficient Factory Lookups

- **What goes wrong:** The application saves memory by sharing objects, but becomes CPU-bound because the factory spends all its time hashing keys to look up flyweights.
- **Why it happens (the mechanism):** If the factory uses a `HashMap` to deduplicate intrinsic states during creation, and the lookup key is a concatenated `String` or complex object, the hashing cost outweighs the memory benefits.
- **How to handle it, and why that works:** Optimize the factory lookup. Use faster hashers (`rustc-hash`), or better yet, resolve the flyweight ID *once* at system startup or asset-load time, rather than querying the factory in a hot loop.
- **Trade-offs of the fix:** Requires lifting asset resolution logic higher in the application architecture.

### Pitfall: Mutating Intrinsic State

- **What goes wrong:** You need to change the color of one specific tree, so you mutate the shared tree mesh. Suddenly, every tree in the forest changes color.
- **Why it happens (the mechanism):** The separation of intrinsic vs. extrinsic state was flawed. A property that you assumed was invariant (color) actually needed to vary per context. 
- **How to handle it, and why that works:** Flyweights must be strictly immutable. Move the mutating property (color) out of the intrinsic state and into the extrinsic Context state.
- **Trade-offs of the fix:** The extrinsic state grows larger, consuming more memory per instance.

### Pitfall: Ghost Flyweights (Arena Leaks)

- **What goes wrong:** You use the Arena index approach, delete thousands of entities, but memory usage never drops.
- **Why it happens (the mechanism):** Indices don't have destructors. When the last `Entity` referencing `mesh_id = 4` is destroyed, the Arena doesn't know. The heavy intrinsic state sits in the `Vec` forever.
- **How to handle it, and why that works:** Implement reference counting at the Arena level, or use generational indices (e.g., `slotmap` crate) and periodically sweep the Arena to remove unreferenced assets. 
- **Trade-offs of the fix:** Reintroduces bookkeeping overhead that the index approach specifically aimed to avoid.

## Design Decisions & Trade-offs

**Indices vs. Lifetimes.**
Using `&'a T` for flyweights provides compile-time guarantee that the flyweight exists, with zero runtime overhead. However, it requires the Context to carry lifetimes, which infects the entire codebase and makes the Contexts hard to store in dynamic collections. Indices defer the check to runtime (via `arena.get(id)`) but keep structs clean and serializable.

## Exercises & Self-Test

1. Measure the performance difference between iterating over 100,000 structs holding `Rc<Data>` vs 100,000 structs holding `usize` indices into a `Vec<Data>`. 
2. Why is a contiguous memory stride crucial for performance in modern CPUs, and how does the Flyweight pattern interact with it?
3. Design an Arena that automatically cleans up unreferenced flyweights using a manual reference count array parallel to the data array.

## Open Questions

- In multi-threaded Rust architectures (like game engines), how do we best share a single read-only Arena across threads without `RwLock` contention?
- When does the memory cost of the `usize` index (8 bytes on 64-bit) outweigh the cost of simply duplicating the data, and how do we benchmark that threshold?

## References

- "Game Programming Patterns" by Robert Nystrom — The definitive guide to the Flyweight pattern in modern Data-Oriented contexts.
- Data-Oriented Design concepts (cache lines, hardware prefetching).
- `slotmap` and `string_interner` crates for practical Rust applications of Arenas and Flyweights.
