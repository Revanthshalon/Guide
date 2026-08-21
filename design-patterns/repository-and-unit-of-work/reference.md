# Repository & Unit of Work — Quick Reference

## One-Liner

**Repository** provides the illusion of an in-memory collection to hide data access mechanics; **Unit of Work** coordinates multiple repositories to commit changes as a single atomic transaction.

## When to Use / When Not

| Use it when | Avoid it when |
| --- | --- |
| Your business logic is complex and must be decoupled from database schema changes. | You are building a simple CRUD app where endpoints just map JSON directly to tables. |
| You need to unit test domain rules instantly without spinning up a real database. | You need to write hyper-optimized analytical SQL queries (use direct queries or CQRS instead). |
| You are coordinating operations across multiple aggregates (requires Unit of Work). | Your logic only ever touches a single row at a time. |

## Structure Sketch

```rust
// 1. The Domain Traits
pub trait UserRepository {
    async fn find(&mut self, id: Uuid) -> Result<User, DomainError>;
    async fn save(&mut self, user: &User) -> Result<(), DomainError>;
}

// 2. The Unit of Work (Transaction Wrapper)
pub struct UnitOfWork<'c> {
    tx: sqlx::Transaction<'c, sqlx::Postgres>,
}

impl<'c> UnitOfWork<'c> {
    // Deref mutably to PgConnection to avoid double-lifetime locks
    pub fn users(&mut self) -> PgUserRepository<'_> {
        PgUserRepository { conn: &mut *self.tx }
    }
    
    pub async fn commit(self) -> Result<(), sqlx::Error> {
        self.tx.commit().await
    }
}

// 3. The Repository Implementation
pub struct PgUserRepository<'a> {
    conn: &'a mut sqlx::PgConnection,
}
```

## Rust Idiom

In Rust, the heavy, state-tracking "Unit of Work" object from classical OOP is unnecessary. Simply wrapping an `sqlx::Transaction` and utilizing Rust's RAII (automatic rollback on `Drop`) is the idiomatic, zero-cost way to achieve atomic commits.

**Critical trick:** Do not pass `&'a mut Transaction<'a, Postgres>` to your repositories. It makes the lifetime invariant, permanently locking the transaction. Instead, have the repository accept a `&'a mut PgConnection` and use `&mut *self.tx` to deref the transaction into a connection.

## Versus

| Confused with | Key difference |
| --- | --- |
| **DAO (Data Access Object)** | DAO models database tables and speaks SQL (e.g., `select_where`). Repository models Domain Aggregates and speaks business rules (e.g., `find_active`). |
| **Active Record** | Active Record binds save/load methods directly onto the domain structs (`user.save()`). Repository separates state (domain structs) from persistence mechanics. |

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| **Generic CRUD Traits** | Write bespoke, domain-specific traits (`find_active`, not `find_by_criteria`). | Repositories slowly recreating SQL WHERE clauses in method signatures. |
| **Leaking DB Errors** | Map infrastructure errors (e.g., `sqlx::Error`) to `DomainError` inside the repository. | Domain code importing `sqlx` just to pattern match on database errors. |
| **N+1 Queries** | Fetch required relations in a single JOIN in the repository's `find` method. | `for` loops in domain logic calling `repo.find()` repeatedly. |

## Rules of Thumb

- A Repository should speak the language of your domain, not your database.
- A Repository manages a single Aggregate Root.
- A Unit of Work spans multiple Repositories but shares a single database transaction.
- If a transaction is dropped in Rust before `commit()` is explicitly called, it automatically rolls back.
- Never let `sqlx` or ORM types leak into your domain logic.

## Key References

- [Outbox Pattern](../../architecture-patterns/outbox-pattern/learning.md) - Often requires a UoW to update state and emit events atomically.
- [Dependency Injection](../dependency-injection/learning.md) - How to inject mock repositories for fast domain testing.
