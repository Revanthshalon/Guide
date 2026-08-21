# Prototype — Learning Notes

## Mental Model

The Prototype pattern specifies the kind of objects to create using a prototypical instance, and creates new objects by copying this prototype. In engineering terms, it solves the problem of instantiating objects that require complex, data-driven initialization. Instead of hard-coding the setup or reading from a database every time, you create one "master" instance and clone it.

In Rust, Prototype maps directly to the `Clone` trait. However, blindly cloning deep object graphs is an anti-pattern in Rust due to hidden heap allocation costs. Idiomatic Rust prototypes use `Arc` or `Cow` to share immutable data cheaply while only allocating memory for the specific fields that must be unique.

## Structure & Participants

### Prototype
- **Role:** An object that can clone itself.
- **In Rust:** A struct implementing `Clone`.

### Client
- **Role:** Creates a new object by asking a prototype to clone itself.
- **In Rust:** Code that holds a prototype and calls `.clone()` on it, often followed by structural updates.

## Idiomatic Rust Implementation & Worked Example

Let's look at building a game entity spawner (e.g., spawning NPCs).

### Stage 0: Naive Deep Cloning (The Anti-Pattern)

The naive approach derives `Clone` on a struct containing heavy heap allocations (`String`, `Vec`). Every time we spawn an NPC, we copy all the dialogue strings entirely in memory.

```rust
#[derive(Clone, Debug)]
pub struct NaiveNpc {
    name: String,
    dialogue: Vec<String>,
    health: u32,
}

// Spawning involves cloning the entire Vec<String>, which is expensive.
```

### Stage 1: `Arc` Optimization for Shared Read-Only Data

Since all "Goblin" NPCs share the same dialogue and it never changes, we should share that data across all clones using `Arc`.

```rust
use std::sync::Arc;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct OptimizedNpc {
    name: String,
    // Shared immutable data uses Arc instead of deep cloning
    dialogue: Arc<Vec<String>>,
    health: u32,
}

pub struct NpcSpawner {
    templates: HashMap<String, OptimizedNpc>,
}

impl NpcSpawner {
    pub fn new() -> Self {
        let mut templates = HashMap::new();
        templates.insert(
            "goblin".to_string(),
            OptimizedNpc {
                name: "Goblin".to_string(),
                dialogue: Arc::new(vec!["Grawr!".to_string(), "Shiny!".to_string()]),
                health: 100,
            }
        );
        Self { templates }
    }

    pub fn spawn(&self, template_id: &str, custom_name: Option<String>) -> Option<OptimizedNpc> {
        // Cloning the Arc only increments a counter, it doesn't copy the Vec!
        let mut npc = self.templates.get(template_id)?.clone();
        
        // Mutate the specific fields for this instance
        if let Some(name) = custom_name {
            npc.name = name;
        }
        Some(npc)
    }
}

// Usage
// let spawner = NpcSpawner::new();
// let bob = spawner.spawn("goblin", Some("Bob".to_string())).unwrap();
```

## When This Pattern Dissolves in Rust

The Prototype pattern dissolves immediately into the standard library `Clone` trait. The architectural intent of "Prototype" is simply "I have an object, I call `.clone()`, and I tweak it."

## Versus

- **Factory:** A factory typically starts from scratch (or a config file) to build a fresh instance. Prototype starts from an existing, fully populated instance in memory.
- **Builder:** Builder constructs an object step-by-step from scratch. Prototype clones a pre-existing blueprint.

## Pitfalls in Depth

### 1. Hidden Allocation Costs
- **What goes wrong:** Performance tanks because `clone()` is silently performing massive deep copies of vectors, strings, and hash maps.
- **Why it happens:** Deriving `Clone` on structs containing heavy heap-allocated types copies the entire heap allocation.
- **How to handle it, and why that works:** Wrap read-only, heavy data in `Arc<T>` or use `Cow<'a, T>`. Cloning an `Arc` is a cheap atomic increment.
- **Trade-offs of the fix:** `Arc` adds atomic overhead and requires managing lifetimes if interior mutability is later needed. `Cow` adds lifetime annotations to the struct.

### 2. Shallow vs Deep Copy Confusion
- **What goes wrong:** You mutate a cloned object, but the original prototype also changes.
- **Why it happens:** If you use `Rc<RefCell<T>>` or `Arc<Mutex<T>>` to optimize cloning, the clone shares the same mutable reference as the prototype.
- **How to handle it, and why that works:** Only share *immutable* data via `Arc<T>`. If the clone needs to mutate the data independently, you *must* perform a deep clone, or use Copy-on-Write techniques.
- **Trade-offs of the fix:** Strict separation between shared static data and unique instance data complicates struct definitions.

### 3. Stale Prototype Data
- **What goes wrong:** Cloned objects start in an invalid state because the prototype was mutated or degraded over time.
- **Why it happens:** The prototype object is kept in a global or mutable state and accidentally modified by a rogue system.
- **How to handle it, and why that works:** Keep prototypes in a read-only registry or encapsulate them strictly so they cannot be mutated after initialization.
- **Trade-offs of the fix:** Requires a dedicated registry struct (like `NpcSpawner`) rather than just passing the prototype around freely.

## Design Decisions & Trade-offs

- **`Clone` vs `Copy`:** Use `Copy` only for small, stack-allocated primitives. Never implement `Copy` for types that manage resources or large structs, as implicit copying will mask performance issues. `Clone` forces the caller to be explicit.
- **Structural Updates:** After cloning, you often need to modify the clone. Use mutable variables (`let mut clone = prototype.clone(); clone.x = 2;`) or combine it with a builder-like method (`prototype.clone().with_x(2)`).

## Exercises & Self-Test

1. **Build Exercise:** Implement a prototype registry for a `Spell` struct in a game. Ensure the heavy particle effect data is shared via `Arc`, while the current damage modifier is unique to each cloned spell.
2. **Build Exercise:** Use the `Cow` (Copy-on-Write) type to store a string that is usually shared with the prototype, but can be modified by the clone without affecting the prototype.
3. Why is `[derive(Clone)]` dangerous on large nested structs?

## Open Questions

- How do you manage the memory overhead of a massive prototype registry that loads thousands of templates at startup?
- Can `Cow` replace `Arc` entirely for prototype optimization?

## References

- [Rust standard library - Clone trait](https://doc.rust-lang.org/std/clone/trait.Clone.html)
- [Rust standard library - Arc](https://doc.rust-lang.org/std/sync/struct.Arc.html)
