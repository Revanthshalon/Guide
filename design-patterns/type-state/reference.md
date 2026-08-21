# Type State — Quick Reference

## One-Liner

Encode an object's state in its type parameter using zero-sized marker structs, shifting invalid state transition errors from runtime panics to compile-time guarantees.

## When to Use / When Not

| Use it when | Avoid it when |
| --- | --- |
| A struct goes through distinct lifecycle phases (e.g., Uninitialized → Ready → Closed). | The state transitions are driven dynamically by unpredictable external data (e.g., network packets). |
| You are writing builders requiring specific combinations of fields. | You need to store many objects of different states in the same array (`Vec`). |
| You want to physically prevent calling certain methods in the wrong state (e.g., hardware drivers). | The number of independent states creates an unmanageable combinatorial explosion. |

## Structure Sketch

```rust
use std::marker::PhantomData;

// States
struct Unlocked;
struct Locked;

// The Type State struct
struct Door<State> {
    _state: PhantomData<State>,
}

impl Door<Unlocked> {
    // Consume self to change type
    fn lock(self) -> Door<Locked> { Door { _state: PhantomData } }
}

impl Door<Locked> {
    fn unlock(self) -> Door<Unlocked> { Door { _state: PhantomData } }
}
```

## Rust Idiom

Leverage Rust's move semantics: by having transition methods take `self` by value, the old state is consumed and invalidated. The caller physically cannot misuse the old state, something impossible in Garbage Collected languages.

## Versus

| Confused with | Key difference |
| --- | --- |
| Runtime State Machine | Runtime state checks `enum` variants and returns `Result`. Type state checks generics and is verified by the compiler. |
| Builder Pattern | Standard builders return `Result<T>` on build. Type State builders ensure `build()` only exists on fully initialized types. |

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| **State Explosion** | Separate orthogonal states into multiple generics: `Session<Auth, Enc>` | Type signatures become noisy and hard to read. |
| **Monomorphization Bloat** | Extract generic-independent logic into a non-generic inner struct. | Code bloat and slower compilation times. |
| **Heterogeneous Storage** | Wrap the generic type in an `enum` | You lose the compile-time guarantees upon extraction. |
| **Confusing Documentation** | Heavily comment transition methods and expected generic parameters | Users will struggle with `expected X, found Y` errors. |

## Rules of Thumb

- If a method checks a boolean flag or enum before doing work, consider if Type State can eliminate that runtime check.
- Use `PhantomData<State>` to hold the generic parameter without consuming memory. Zero-sized types cost absolutely nothing at runtime.
- Methods that change state **must** take `self` (not `&mut self`), forcing the caller to use the returned new type.

## Key References

- [Typestate pattern in Rust](https://cliffle.com/blog/rust-typestate/) by Cliff Biffle.
