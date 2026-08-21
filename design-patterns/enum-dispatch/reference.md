# Enum Dispatch — Quick Reference

## One-Liner

Replace dynamic trait objects (`Box<dyn Trait>`) with an enum of concrete types to achieve polymorphism with static dispatch, better cache locality, and no heap allocation.

## When to Use / When Not

| Use it when | Avoid it when |
| --- | --- |
| The set of types is known at compile time (closed). | The trait is a public API expecting external implementations. |
| You want to store polymorphic types contiguously in memory (`Vec<Enum>`). | The variants are drastically different in size (unless boxed). |
| Performance (inlining, no vtable lookups) is critical. | You have dozens of variants and the `match` boilerplate is prohibitive. |

## Structure Sketch

```rust
trait Process { fn process(&self); }

struct A; impl Process for A { fn process(&self) {} }
struct B; impl Process for B { fn process(&self) {} }

// The Dispatch Enum
enum Item { A(A), B(B) }

// The Static Dispatch implementation
impl Process for Item {
    fn process(&self) {
        match self {
            Item::A(a) => a.process(),
            Item::B(b) => b.process(),
        }
    }
}
```

## Rust Idiom

Instead of `Vec<Box<dyn Trait>>`, use `Vec<Enum>`. Consider using the `enum_dispatch` crate to automatically generate the trait implementation for the enum.

## Versus

| Confused with | Key difference |
| --- | --- |
| Trait Objects (`dyn Trait`) | `dyn Trait` is open/extensible but incurs heap/vtable costs. Enums are closed but statically dispatched. |
| Visitor Pattern | Visitor uses double-dispatch to recover types. Enums use pattern matching, making Visitor largely obsolete (except for cross-crate extensions). |

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| **Fat Enums** | `Box` the abnormally large variants: `Big(Box<HugeStruct>)` | The enum size is the size of its largest variant + discriminant. |
| **Delegation Boilerplate** | Use `enum_dispatch` or `delegate` macros | Proc-macros increase compilation time. |
| **Open/Closed Violation** | Add a `Custom(Box<dyn Trait>)` escape hatch | Escape hatches increase the size of the enum to at least two pointers. |

## Rules of Thumb

- Default to Enum Dispatch over `dyn Trait` unless you specifically need an extensible plugin system.
- Check enum sizes with `std::mem::size_of::<MyEnum>()`. If it's over 100 bytes or heavily skewed, look for variants to Box.
- Let exhaustiveness checking guide your refactoring: adding a variant is safe because the compiler finds all the unhandled match arms.

## Key References

- [enum_dispatch crate documentation](https://docs.rs/enum_dispatch/latest/enum_dispatch/)
