# Command — Learning Notes

## Mental Model

**When you call a function, the thread of execution jumps immediately to the target. But what if the time of execution, the location of execution, or the system that *triggers* the execution is different from the system that *understands* the execution?** The Command pattern disconnects *what* to do from *when* and *where* to do it by reifying the method call into data.

It encapsulates the instruction (the verb) and its arguments into a standalone struct or enum variant. By turning a verb into a noun, you unlock abilities that direct calls lack: you can store them in a queue, delay them, log them, serialize them over a network (RPC), or reverse them for an undo system.

## Structure & Participants

### Command (or Message)
- **Role:** The reified action, containing all the arguments necessary to perform the work.
- **In classic OOP:** An interface with an `execute()` method.
- **In Rust:** An `enum` variant (most idiomatic), a closure (`Box<dyn FnOnce()>`), or a trait.

### Invoker (or Queue / Scheduler)
- **Role:** The component that decides *when* to execute the command.
- **In Rust:** A task queue, an undo stack, or a UI event loop. It holds the commands but doesn't implement the business logic.

### Receiver (or Context)
- **Role:** The target state that the command mutates when executed.
- **In Rust:** Any domain struct (e.g., `Document`, `DatabaseConn`). Crucially, in Rust, the Receiver is usually *passed into* the command upon execution, rather than being stored inside it.

## Idiomatic Rust Implementation

In Rust, the Command pattern is heavily influenced by the language's preference for data over behavior. 

### 1. The Enum Pattern (Data-Driven Commands)
If you know all possible commands at compile time, an `enum` is the most idiomatic, performant, and robust approach. It gives you exhaustive pattern matching, trivial serialization, and avoids the heap allocation of dynamic dispatch. This strongly overlaps with Event Sourcing.

```rust
use std::collections::VecDeque;

// 1. The Receiver
pub struct Document {
    text: String,
}

// 2. The Command (Enum)
// Notice how it holds data, not references to the Document.
#[derive(Debug, Clone)]
pub enum EditorCmd {
    Insert { position: usize, text: String },
    Backspace { position: usize, count: usize },
}

// 3. The Invoker (Queue)
pub struct EventLoop {
    queue: VecDeque<EditorCmd>,
    doc: Document,
}

impl EventLoop {
    pub fn execute_all(&mut self) {
        while let Some(cmd) = self.queue.pop_front() {
            // The command is just data; the invoker applies it to the receiver.
            match cmd {
                EditorCmd::Insert { position, text } => {
                    self.doc.text.insert_str(position, &text);
                }
                EditorCmd::Backspace { position, count } => {
                    let end = position + count;
                    self.doc.text.replace_range(position..end, "");
                }
            }
        }
    }
}
```

### 2. Closures (One-way Tasks)
If you don't need undo or serialization, and you just want to defer work (e.g., a thread pool), a closure *is* a Command.

```rust
pub struct Worker {
    // FnOnce means the command consumes its captures and runs exactly once.
    // Send allows it to cross thread boundaries.
    tasks: Vec<Box<dyn FnOnce() + Send>>,
}

impl Worker {
    pub fn push<F>(&mut self, task: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.tasks.push(Box::new(task));
    }
}
```

## When This Pattern Dissolves in Rust

For simple delayed execution, the pattern **dissolves entirely into closures**. Standard library types like `std::thread::spawn` or thread pools use `Box<dyn FnOnce() + Send>` directly. 

When you need an Undo/Redo stack or audit logging, the pattern does not dissolve, but it fundamentally shifts from **Trait Objects (OOP)** to **Enums (Functional/Data-driven)** to accommodate Rust's ownership model and to leverage `serde` for serialization.

## Worked Example

Let's build a robust Undo/Redo stack for a text editor.

**Stage 0: Direct Mutation (No History)**
```rust
impl Document {
    fn insert(&mut self, pos: usize, text: &str) { /* ... */ }
}
```
Direct mutation is fine, but when the user hits `Ctrl+Z`, there's no record of what just happened. We need to save the operations.

**Stage 1: The Broken OOP Port (The `&mut` Trap)**
A direct transliteration of Java's Command pattern tries to make the command fully self-contained by holding a reference to the receiver:
```rust
// ❌ DO NOT DO THIS
pub struct InsertCommand<'a> {
    doc: &'a mut Document, // Exclusive borrow!
    text: String,
}
```
If you push `InsertCommand` into an undo `Vec`, the `Vec` now holds an exclusive (`&mut`) borrow of `Document`. The compiler will instantly reject any attempt to create a second command or access the document, because you cannot alias mutable references. 

**Stage 2: Passing the Context**
The fix is to separate the *payload* from the *receiver*. The command holds the data needed to perform the action, and the invoker passes the `&mut Document` into the `execute` method at runtime.

```rust
pub trait Command {
    fn execute(&mut self, doc: &mut String);
    fn undo(&mut self, doc: &mut String);
}

pub struct InsertText {
    position: usize,
    text: String,
}

impl Command for InsertText {
    fn execute(&mut self, doc: &mut String) {
        doc.insert_str(self.position, &self.text);
    }
    
    fn undo(&mut self, doc: &mut String) {
        let end = self.position + self.text.len();
        doc.replace_range(self.position..end, "");
    }
}
```

**Stage 3: The Complete Undo Stack**
Now the Invoker (`History`) holds the commands and owns the Receiver (`String`). It passes the receiver to the commands on demand.

```rust
pub struct History {
    undo_stack: Vec<Box<dyn Command>>,
    redo_stack: Vec<Box<dyn Command>>,
    state: String,
}

impl History {
    pub fn execute(&mut self, mut cmd: Box<dyn Command>) {
        cmd.execute(&mut self.state);
        self.undo_stack.push(cmd);
        self.redo_stack.clear(); // New actions invalidate redo history
    }

    pub fn undo(&mut self) {
        if let Some(mut cmd) = self.undo_stack.pop() {
            cmd.undo(&mut self.state);
            self.redo_stack.push(cmd);
        }
    }
}
```
This compiles perfectly, enforces exclusive access only at the moment of execution, and forms a robust undo tree.

## Versus

- **Command vs Strategy:** 
  - *Strategy* encapsulates an *algorithm* (how to compute something) and returns a result. It represents a pluggable behavior.
  - *Command* encapsulates a *request* (what to do) and its arguments. It represents a reified event.
- **Command vs Event Sourcing:** 
  - *Command* is the intent to change state (can be rejected).
  - *Event Sourcing* records the fact that state *has* changed (cannot be rejected). Enum-based Commands are often the building blocks of an Event Sourcing system.
- **Command vs Memento:** 
  - *Command* implements undo by reversing a *delta* (e.g., "insert 'a'" -> "delete 'a'").
  - *Memento* implements undo by restoring a full *snapshot* of the state.

## Pitfalls in Depth

### Pitfall: The `&mut` Self-Containment Trap

- **What goes wrong:** You build a Command struct that holds a `&mut Receiver`. You try to push multiple commands into an undo `Vec`. The borrow checker throws a massive error complaining about multiple mutable borrows.
- **Why it happens (the mechanism):** Rust strictly enforces that only one active mutable reference to a piece of data can exist at a time. An undo history holding multiple commands, which each hold a `&mut` to the same document, violates aliasing rules.
- **How to handle it, and why that works:** The Command must store only the *parameters* of the operation (the payload). The `Invoker` (the history stack) must own or hold the lock on the `Receiver`. When it's time to execute or undo, the Invoker passes `&mut Receiver` as an argument to `cmd.execute(receiver)`.
- **Trade-offs of the fix:** Commands are no longer fully self-contained. The Invoker is now coupled to the type of the Receiver, because it has to pass it along.

### Pitfall: State Desynchronization (Inexact Undo)

- **What goes wrong:** A command (e.g., `Delete { index: 5, len: 10 }`) is added to the undo stack. Later, an external process (like an autosave formatter or a collaborative sync) modifies the document. The user clicks "Undo", and the command blindly restores text at index 5, but the document has shifted. The document is corrupted.
- **Why it happens (the mechanism):** Delta-based undo assumes strict serialization and total isolation. It assumes that `state_n` can only be reached by applying `command_n` to `state_n-1`. If state changes out-of-band, the inverse delta applies to the wrong baseline.
- **How to handle it, and why that works:** If you use delta-based undo, you must lock down the receiver so that *every single mutation* goes through the command queue. If that's impossible (e.g., multiplayer collaborative apps), you must abandon simple Command undo and use Operational Transformation (OT), CRDTs, or fallback to the Memento pattern (saving full snapshots).
- **Trade-offs of the fix:** Forcing all mutations through a command queue can be bottlenecking and boilerplate-heavy. Mementos are memory-intensive.

### Pitfall: Polymorphic Serialization (`dyn Trait` Opacity)

- **What goes wrong:** You build an undo history using `Vec<Box<dyn Command>>`. You want to save the user's session to a file so they can resume tomorrow. You try to `#[derive(Serialize)]` on your `History` struct, and the compiler rejects it.
- **Why it happens (the mechanism):** `dyn Trait` erases the concrete type at compile time. Serde cannot serialize an erased type because it doesn't know what fields it contains or what type tag to write to the JSON so it can be deserialized later.
- **How to handle it, and why that works:** Switch from OOP `dyn Trait` commands to an `enum` of commands. Enums trivially derive `Serialize` and `Deserialize` because the compiler knows all variants. If you absolutely need polymorphic, open-ended commands (e.g., a plugin system), use a crate like `typetag` which injects type metadata into the serialization payload.
- **Trade-offs of the fix:** Enums are closed; plugins cannot add new variants to your enum. `typetag` relies on dynamic dispatch, macro magic, and forces a specific serialization format.

## Design Decisions & Trade-offs

**Enums vs Trait Objects:** Always default to Enums. They serialize easily, skip dynamic dispatch, and enforce a clear boundary between data (the command) and behavior (the interpreter). Reach for Traits only if you are building an extensible architecture where downstream users need to inject custom commands without modifying your source code.

**Delta vs Snapshot (Memento):** Reversing a delta (`undo`) takes very little memory, but the logic to compute a perfect inverse is mathematically fragile (especially for destructive operations). Saving a full snapshot (Memento) is memory-intensive but guaranteed to be correct.

**Fallibility in Execution:** If `execute()` can fail halfway through mutating the receiver, the receiver is left corrupted, but the command might still be recorded. Commands should ideally compute the required changes on a clone of the state or use transactional operations so that failures leave the baseline state untouched.

## Exercises & Self-Test

1. Explain why storing `&mut Receiver` inside a `Command` struct makes an undo history impossible to compile in Rust. What is the structural fix?
2. If an application uses an `enum` for commands, where does the `execute()` logic live? How does this differ from the Trait-based approach?
3. What happens to a delta-based undo stack if a background thread mutates the Receiver directly? 
4. **Design Exercise:** A standard Command deletes a user account. How do you implement `undo()` for a destructive operation? Does the Command need to store the deleted data, or just the ID?
5. **Build Exercise:** Implement the `enum`-based Undo stack for a basic text editor. Ensure that the enum derives `Serialize` and `Deserialize` using `serde`.

## References

- [Rust Design Patterns - Command](https://rust-unofficial.github.io/patterns/patterns/behavioural/command.html) — Further examples of closures as commands.
- [Event Sourcing](https://martinfowler.com/eaaDev/EventSourcing.html) — Martin Fowler's guide. In Rust, Event Sourcing is fundamentally the Enum-based Command pattern persisted to an append-only log.
- [Serde documentation on Enum representations](https://serde.rs/enum-representations.html) — Crucial when serializing enum-based commands for RPC or disk persistence.
