# State Machine — Quick Reference

## One-Liner

Formalize conditional logic by representing each distinct condition as a State, binding data and valid actions strictly to that specific state.

## When to Use / When Not

| Use it when | Avoid it when |
| --- | --- |
| An object has a strict lifecycle with distinct phases (e.g., Draft -> Review -> Published). | The behavior changes based on orthogonal boolean flags that don't represent distinct phases. |
| You have complex `if/else` logic checking `state == X` in every method. | The states and transitions are simple enough for a single enum without distinct payloads. |

## Structure Sketch

```rust
// Data is tightly bound to the state it belongs to
pub struct CartData { pub items: Vec<String> }
pub struct PaidData { pub items: Vec<String>, pub receipt_id: String }

pub enum OrderState {
    Cart(CartData),
    Paid(PaidData),
}

// Transitions consume the old state and return the new one
impl OrderState {
    pub fn checkout(self, receipt_id: String) -> Result<Self, (Self, &'static str)> {
        match self {
            OrderState::Cart(data) => Ok(OrderState::Paid(PaidData { 
                items: data.items, 
                receipt_id 
            })),
            // Explicitly reject invalid transitions instead of silent no-ops
            other => Err((other, "Cannot checkout: Not in Cart state")), 
        }
    }
}
```

## Rust Idiom

- **Use Enums for Runtime:** Rust enums with data payloads completely supersede the OOP class-based State pattern for dynamic, runtime state machines.
- **Use Typestate for Compile-Time:** If states progress deterministically, encode them as generic type parameters (`Connection<Closed>`).
- **Consume `self` or use `Option::take`:** Prefer state transition methods that take `self` (ownership). When mutating a context struct in-place, wrap the state in `Option` and use `.take()` to temporarily move it out for transition.

## Versus

| Confused with | Key difference |
| --- | --- |
| **Strategy** | Strategy swaps behavior from the outside based on config. State transitions itself from the inside based on lifecycle. |
| **Type State** | State Machine resolves transitions at runtime (using `match`). Type State resolves transitions at compile-time (using distinct types). |

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| **State Data Bleed** | Put state-specific data *inside* the enum variant payload, not in a top-level wrapper struct. | `Option` fields in the outer struct that are only "valid" in certain states. |
| **Borrow Checker on Transitions** | Don't mutate `&mut self.state` directly. Wrap state in `Option`, use `.take()`, transition, and put it back. | Ergonomic friction of dealing with the `Option` wrapper. |
| **Silent Failures** | Return `Result<Self, Error>` on transitions. Don't use a wildcard `_ => self` that silently swallows invalid operations. | Verbosity for callers who must now handle the `Result`. |

## Rules of Thumb

- If a field is only valid in one specific state, it must live inside the enum variant (or typestate struct) for that state.
- If you find yourself writing `if let State::X = self.state` in multiple methods to access data, you need to push that logic down into the state transition itself.
- For open-ended, plugin-based state systems where states are unknown at compile time, fall back to dynamic dispatch (`Box<dyn State>`).

## Key References

- [Rust Design Patterns - Typestate](https://rust-unofficial.github.io/patterns/patterns/creational/builder.html)
- [Hoverbear - Rust State Machine Patterns](https://hoverbear.org/blog/rust-state-machine-pattern/) 
