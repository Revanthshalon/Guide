# Repository & Unit of Work — Learning Notes

## Mental Model

**Data access should feel like working with an in-memory collection, and business operations should fail or succeed as a single atomic unit.**

Business logic and data storage mechanics change for different reasons and at different rates. If you embed raw SQL queries directly inside your domain logic, every database schema change breaks your domain layer, and you cannot test your business rules without spinning up a real database.

The **Repository** pattern provides the *illusion* of an in-memory collection. It acts as a boundary, allowing domain objects to fetch and save data without knowing *how* that data is persisted.

The **Unit of Work (UoW)** pattern provides the *illusion* of a single snapshot in time. A business operation often spans multiple repositories (e.g., deducting a user's balance and recording an event). The UoW coordinates these changes, ensuring they share the same underlying database transaction so they either commit entirely or roll back as one.

## Structure & Participants

### Repository

- **Role:** Abstracts data retrieval and persistence for a specific Domain Aggregate (e.g., `User`, `Order`).
- **In classic OOP:** An interface defining CRUD operations, backed by a concrete class wrapping an ORM.
- **In Rust:** A `trait` defining bespoke, domain-specific operations (not just generic CRUD), taking a mutable reference to a database connection, returning domain types and domain errors.

### Unit of Work (UoW)

- **Role:** Tracks changes across multiple repositories and commits them as a single atomic transaction.
- **In classic OOP:** A complex stateful object (like Hibernate's `Session`) that tracks dirty/new/deleted entities in memory and flushes them to the database on `commit()`.
- **In Rust:** A lightweight struct holding a database transaction (e.g., `sqlx::Transaction`). It exposes methods to borrow the transaction and create repositories that operate on it.

## Idiomatic Rust Implementation

In Rust, the classic Unit of Work simplifies dramatically. You do not need a complex object tracking "dirty" entities. Instead, a UoW simply wraps an `sqlx::Transaction`.

The most critical realization is how to handle lifetimes. A naive implementation attempts to pass `&mut Transaction` into the repository, which often leads to broken lifetimes where the transaction is permanently locked by the first borrow. The idiomatic solution takes advantage of `DerefMut`: an `sqlx::Transaction` dereferences into an `sqlx::PgConnection`. By having the repository take a `&mut PgConnection`, we sidestep complex double-lifetimes entirely.

```rust
use sqlx::{PgConnection, PgPool, Postgres, Transaction};
use uuid::Uuid;

// 1. The Domain Types
pub struct User { pub id: Uuid, pub balance: i64 }
pub struct OutboxEvent { pub id: Uuid, pub payload: String }

// 2. The Repository Traits
// (Note: Using native async traits, available in Rust 1.75+)
pub trait UserRepository {
    async fn find_by_id(&mut self, id: Uuid) -> Result<Option<User>, sqlx::Error>;
    async fn update(&mut self, user: &User) -> Result<(), sqlx::Error>;
}

pub trait OutboxRepository {
    async fn insert(&mut self, event: &OutboxEvent) -> Result<(), sqlx::Error>;
}

// 3. The Unit of Work implementation
pub struct PgUnitOfWork<'c> {
    tx: Transaction<'c, Postgres>,
}

impl<'c> PgUnitOfWork<'c> {
    pub async fn begin(pool: &'c PgPool) -> Result<Self, sqlx::Error> {
        let tx = pool.begin().await?;
        Ok(Self { tx })
    }

    pub async fn commit(self) -> Result<(), sqlx::Error> {
        self.tx.commit().await
    }

    // `self.tx` derefs to `PgConnection`. 
    // We only borrow the connection for the lifetime of the repository accessor!
    pub fn users(&mut self) -> PgUserRepository<'_> {
        PgUserRepository { conn: &mut *self.tx }
    }

    pub fn outbox(&mut self) -> PgOutboxRepository<'_> {
        PgOutboxRepository { conn: &mut *self.tx }
    }
}

// 4. Concrete Repository implementations
// Notice there is only one lifetime ('a): the duration of the method call.
pub struct PgUserRepository<'a> {
    conn: &'a mut PgConnection,
}

impl<'a> UserRepository for PgUserRepository<'a> {
    async fn find_by_id(&mut self, _id: Uuid) -> Result<Option<User>, sqlx::Error> {
        // sqlx::query!("SELECT * FROM users WHERE id = $1", id)
        //     .fetch_optional(&mut *self.conn).await
        Ok(None)
    }

    async fn update(&mut self, _user: &User) -> Result<(), sqlx::Error> {
        Ok(())
    }
}

pub struct PgOutboxRepository<'a> {
    conn: &'a mut PgConnection,
}

impl<'a> OutboxRepository for PgOutboxRepository<'a> {
    async fn insert(&mut self, _event: &OutboxEvent) -> Result<(), sqlx::Error> {
        Ok(())
    }
}
```

Notice the ownership mechanics: `PgUnitOfWork` holds the actual `Transaction`. The repository accessors (`users()`, `outbox()`) borrow the connection mutably. Because Rust enforces exclusive mutable references (`&mut`), you can only use one repository at a time from a single UoW, perfectly matching the sequential nature of most SQL connections.

## When This Pattern Dissolves in Rust

The classical ORM-heavy Unit of Work completely dissolves in Rust. Rust does not have a widespread culture of "transparent" ORMs that track dirty object state in memory. 

Instead of a complex UoW object that diffs entities, the "Unit of Work" in Rust devolves down to **just passing a transaction around**. The database itself does the heavy lifting of atomic commits. 

The Repository pattern, however, remains highly relevant for testing (mocking data access) and isolating business logic.

## Worked Example

Let's look at how this connects to the [Outbox Pattern](../../architecture-patterns/outbox-pattern/learning.md). We need to deduct a user's balance and emit an event, atomically.

**Stage 0: No Unit of Work (The Problem)**

If we pass individual repositories into our service, they manage their own connections:

```rust
async fn process_payment(users: &mut impl UserRepository, outbox: &mut impl OutboxRepository) {
    let mut user = users.find_by_id(id).await.unwrap();
    user.balance -= 100;
    users.update(&user).await.unwrap(); // <-- Commits immediately
    
    // If the server crashes HERE, the user lost money but the event was never sent!
    outbox.insert(&event).await.unwrap();
}
```

**Stage 1: Unit of Work (The Solution)**

By grouping the repositories under a single transaction boundary, the operation becomes atomic:

```rust
async fn process_payment(
    pool: &PgPool, 
    user_id: Uuid, 
    amount: i64
) -> Result<(), AppError> {
    // 1. Begin the Unit of Work (Transaction starts)
    let mut uow = PgUnitOfWork::begin(pool).await?;

    // 2. Read state via Repository
    let mut user = uow.users().find_by_id(user_id).await?
        .ok_or(AppError::UserNotFound)?;

    // 3. Apply Domain Logic
    if user.balance < amount {
        return Err(AppError::InsufficientFunds);
    }
    user.balance -= amount;

    // 4. Persist state and event
    uow.users().update(&user).await?;
    
    let event = OutboxEvent { id: Uuid::new_v4(), payload: "PaymentProcessed".into() };
    uow.outbox().insert(&event).await?;

    // 5. Commit
    uow.commit().await?;

    Ok(())
}
```

If any step fails, the `?` operator returns early, dropping the `PgUnitOfWork`. When an `sqlx::Transaction` is dropped without being committed, its `Drop` implementation automatically triggers a rollback. This RAII behavior is a massive safety net in Rust.

## Versus

- **DAO (Data Access Object):** A DAO maps 1:1 with database tables and exposes SQL-centric concepts. A Repository maps to a Domain Aggregate and speaks the language of the domain (e.g., `find_active_users` instead of `select_where_status_is_1`).
- **Active Record:** In Active Record (like Ruby on Rails), entities know how to save themselves (`user.save()`). In Repository, entities are pure data/logic structures, and the Repository handles persistence. Active Record mixes business logic with data access; Repository separates them.
- **Direct Queries:** Just writing `sqlx::query!(...).execute(pool)` everywhere. Simple, but tightly couples your domain logic to the database schema, making it hard to test business rules without a real database.

## Pitfalls in Depth

### Pitfall: Over-abstracting into Generic Repositories

- **What goes wrong:** You define `trait Repository<T> { async fn get(id: Uuid) -> T; async fn save(t: &T); }` to save typing. Soon, you need to query users by email, so you add a complex `find_by_criteria` method that leaks database-specific concepts (like SQL WHERE clauses) into the domain layer.
- **Why it happens (the mechanism):** Chasing DRY (Don't Repeat Yourself) over clarity. Classic OOP frameworks do this heavily, and developers transliterate it to Rust, assuming a generic interface is "cleaner".
- **How to handle it, and why that works:** Write bespoke, domain-specific traits. `trait UserRepository { async fn find_by_email(...); async fn deactivate_user(...); }`. The repetition is trivial; the clarity and decoupling are immense. Your domain logic now dictates the access patterns it needs.
- **Trade-offs of the fix:** More boilerplate. You have to write explicit traits for every aggregate rather than relying on a generic `impl<T> Repository<T>`.

### Pitfall: Leaking Database Specifics in the Trait

- **What goes wrong:** Your repository trait returns `Result<User, sqlx::Error>`. Your domain logic now has to import `sqlx` just to handle errors (e.g., checking for a unique constraint violation).
- **Why it happens (the mechanism):** Returning the underlying driver's error type is the path of least resistance. It requires zero mapping code.
- **How to handle it, and why that works:** The trait should return a domain-specific error enum (e.g., `Result<User, DomainError>`). The concrete repository implementation must map the infrastructure error to the domain error (e.g., mapping a Postgres unique constraint violation on the email column to `DomainError::EmailAlreadyExists`).
- **Trade-offs of the fix:** You must write and maintain error mapping code, which can be tedious when dealing with raw database string codes (like Postgres's `23505`).

### Pitfall: The Hidden N+1 Problem

- **What goes wrong:** You load an `Order`, and then loop over its item IDs, calling `item_repo.find(id)` for each one. The database is hit hundreds of times for a single business operation, killing performance.
- **Why it happens (the mechanism):** The repository successfully provides the *illusion* of a local collection, causing developers to forget the cost of the underlying network boundaries. In-memory `find()` is nanoseconds; network `find()` is milliseconds.
- **How to handle it, and why that works:** Design aggregate roots carefully. If an `Order` always needs its `Items` to enforce business invariants, the `OrderRepository::find` should fetch both the order and its items in a single JOIN, returning a fully populated `Order` struct. Alternatively, add batch methods: `item_repo.find_many(ids)`.
- **Trade-offs of the fix:** Fetching relations eagerly via JOINs increases memory overhead. If your domain logic only needed the order's status and not its items, you just wasted memory and bandwidth fetching data you didn't use.

## Design Decisions & Trade-offs

**Async Traits vs `#[async_trait]`**
In Rust 1.75+, native `async fn` in traits is available. However, native async traits do not currently enforce `Send` bounds on the returned Futures by default. If your Unit of Work or Repositories are used in a multithreaded runtime like Tokio, you may encounter complex compiler errors when trying to hold them across `.await` points. If you rely heavily on dependency injection via dynamic dispatch (`Box<dyn UserRepository>`), you might still prefer the `#[async_trait]` crate, which explicitly bounds the Future with `Send` and boxes it, sacrificing a tiny bit of performance for ergonomics.

**Closure-based UoW vs Struct-based UoW**
Instead of managing the commit/rollback manually, you can expose a method that takes a closure:
```rust
async fn with_uow<F, Fut, R, E>(&self, f: F) -> Result<R, E>
where
    F: FnOnce(PgUnitOfWork<'_>) -> Fut,
    Fut: Future<Output = Result<R, E>>
```
This guarantees the transaction is closed and removes the risk of forgetting to call `.commit()`. However, dealing with async closures and higher-ranked trait bounds (`for<'a>`) in Rust can be unergonomic and complex for team members to read. The struct-based approach with explicit `.commit()` and RAII rollback on drop is usually preferred for its simplicity.

## Exercises & Self-Test

1. How does a Repository differ from a DAO? Give an example of a method name that belongs in a DAO but not a Repository.
2. Why is passing a `&mut sqlx::Transaction` to a repository a sufficient implementation of a Unit of Work in Rust, compared to state-tracking UoW objects in other languages?
3. What happens in the Rust implementation if `process_payment` returns an error before `uow.commit().await` is called?
4. **Design Exercise:** Map out the domain errors for a `UserRepository`. How would you map a Postgres foreign key violation (e.g., trying to assign a user to a non-existent `Role`) into a `DomainError`?
5. **Build Exercise:** Implement an in-memory version of `UserRepository` using a `std::collections::HashMap`. Write a unit test for `process_payment` passing in your in-memory repository to prove that the domain logic can be tested instantly without a database.

## Open Questions

- Is there a way to enforce `Send` bounds on native async traits in Rust without waiting for the stabilization of `return_type_notation` (RTN)?
- For read-heavy applications, at what point does the abstraction overhead of Repositories justify bypassing them for CQRS-style direct queries?

## References

- Martin Fowler, ["Repository"](https://martinfowler.com/eaaCatalog/repository.html) and ["Unit of Work"](https://martinfowler.com/eaaCatalog/unitOfWork.html) — Canonical definitions.
- Related topics in this repo: [Dependency Injection](../dependency-injection/learning.md) (how to pass the repos), [Outbox Pattern](../../architecture-patterns/outbox-pattern/learning.md) (the prime use-case for a UoW).
