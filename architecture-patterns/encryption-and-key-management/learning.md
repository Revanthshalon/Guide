# Encryption & Key Management — Learning Notes

## Mental Model

**Encryption never eliminates a secret — it exchanges one. Encrypt a 10 GB database with AES and you no longer have a 10 GB secret; you have a 32-byte one: the key.** The entire discipline of key management is what you do with that exchanged secret, and every architecture in this document is an answer to the recursive question it raises: *the data is protected by a key — what protects the key?*

Follow that recursion and the whole field unfolds:

1. Data is encrypted with a **data encryption key (DEK)**. Where does the DEK live? Next to the data? Then anyone who steals the disk has both — you've built a lock with the key taped to it.
2. So encrypt the DEK with another key — a **key encryption key (KEK)**. That's **envelope encryption**: the data wrapped in a key, the key wrapped in another key, like a letter in an envelope. Now the stolen disk yields ciphertext plus a *wrapped* (useless) DEK.
3. What protects the KEK? Another KEK, perhaps — a **key hierarchy** — but this can't recurse forever. It terminates at a **root of trust**: a key that is never written down anywhere software can casually read — held in a hardware security module (HSM), split among humans (Shamir's secret sharing), or provided by a cloud KMS whose own root lives in *their* HSMs.

The payoff of this structure is not just theft-resistance — it's that it makes the three operationally impossible things possible:

- **Rotation without re-encryption:** to rotate, re-wrap the 32-byte DEKs under a new KEK. The 10 GB of data is never touched. Without envelopes, rotating a key means re-encrypting everything it ever protected.
- **Deletion without deletion (crypto-shredding):** destroy one user's DEK and their data — across every table, backup, and replica — becomes noise simultaneously. This is the mechanism [event sourcing](../event-sourcing/learning.md) leans on for GDPR erasure in immutable logs.
- **Centralized control with distributed data:** DEKs travel with the data they protect (wrapped, inert); the KEK never leaves one guarded place (KMS/OpenBao/HSM). The guarded place sees only 32-byte keys, never your data — small, auditable, and its compromise surface is tiny compared to "every service holds the master key."

The second half of the mental model is about *scope*: encryption at rest, in transit, and in use protect against different attackers (stolen disk ≠ network eavesdropper ≠ compromised host), and **encryption at rest below the application (disk/database-level) does nothing against a compromised application** — the app reads plaintext, so an attacker inside the app does too. Application-layer encryption (the envelope machinery above, wielded per-field or per-user) is what changes that. Deciding *which attacker you're defending against* is the first design act; everything else follows from it.

## Core Concepts

### Symmetric encryption and AEAD

- **What it is:** One key both encrypts and decrypts. The modern form is **AEAD** (Authenticated Encryption with Associated Data — AES-256-GCM, ChaCha20-Poly1305): it produces ciphertext *plus an authentication tag*, so tampering is detected at decrypt time, and it can bind unencrypted context (the "associated data" — e.g. the row's user id) so ciphertext can't be cut-and-pasted between records.
- **Why it exists:** Symmetric is the workhorse — fast (AES-NI hardware gives GB/s), small keys (32 bytes), no size limits. Plain unauthenticated encryption (AES-CBC without a MAC) is obsolete because malleable ciphertext enables entire attack classes (padding oracles); AEAD closes that by construction. Always AEAD, never "just AES."
- **Example:** `AES-256-GCM(key, nonce, plaintext, aad="user:42")` → `(ciphertext, tag)`. Decrypting with the wrong key, a flipped bit, or `aad="user:43"` fails loudly rather than returning garbage — the difference between an integrity guarantee and a prayer. The **nonce** (number-used-once) is the sharp edge: GCM catastrophically fails if a (key, nonce) pair repeats (see pitfalls).

### Asymmetric encryption and hybrid schemes

- **What it is:** A key *pair* — public key encrypts (or verifies), private key decrypts (or signs). RSA and elliptic-curve (X25519 for exchange, Ed25519 for signatures) are the families. Asymmetric is slow and size-limited, so real systems are **hybrid**: asymmetric to establish or wrap a symmetric key, symmetric for the bulk data — TLS does exactly this, and envelope encryption is the same shape (the KEK wraps the DEK; the DEK does the work).
- **Why it exists:** It solves the problem symmetric can't: establishing trust with a party you share no secret with, and separating *who can write* from *who can read* (anyone can encrypt to a public key; only the private key's holder reads).
- **Example:** A backup pipeline encrypts archives to a public key held everywhere; the private key sits offline in an HSM and is used only during disaster recovery. Compromising the entire backup fleet yields the ability to *add* backups, not read them — an asymmetry no symmetric design can offer.

### DEK, KEK, and the envelope

- **What it is:** The two-tier key structure: a fresh **DEK** per object/record/user encrypts data locally; the **KEK**, resident only in the KMS/OpenBao/HSM, wraps each DEK; stored beside each piece of data is `(ciphertext, wrapped_DEK, kek_version)`. Decryption: send the wrapped DEK to the KMS (one small call), get the DEK, decrypt locally, discard the DEK.
- **Why it exists:** Three forces converge. *Performance:* the KMS round-trip touches 32 bytes, never your 10 GB — bulk crypto stays local and fast. *Blast radius:* one DEK per user/object means one compromised DEK exposes one object. *Nonce safety:* fresh DEK per object resets the nonce space, making the GCM repeat-nonce catastrophe structurally unlikely instead of procedurally avoided.
- **Example:** Storing a document: generate random 32-byte DEK → `AES-GCM(DEK, doc)` locally → `KMS.encrypt(kek_id, DEK)` → store `(ct, wrapped_dek, v3)`, zeroize DEK from memory. The KMS saw 32 bytes; the document never crossed the wire; the row is self-contained and survives being copied to any replica or backup.

### Key hierarchy, root of trust, and unsealing

- **What it is:** The full chain: root key (HSM / cloud KMS / Shamir-split) → wraps the KMS's internal keys → wrap KEKs → wrap DEKs → protect data. Each layer's key never leaves its layer's guardian. **Unsealing** is the bootstrap act at the bottom: OpenBao at rest holds everything encrypted under a root key it *doesn't store*; starting it requires reconstructing that key — from a quorum of human-held **Shamir shares** (k-of-n key splitting: any 3 of 5 shares reconstruct, 2 reveal nothing) or from a cloud KMS (**auto-unseal**), which moves the recursion's terminus into the cloud provider's HSMs.
- **Why it exists:** The recursion must end somewhere that software compromise alone can't reach — hardware that won't export keys, or a quorum of humans. Everything above that point is rebuildable ciphertext; the design goal is that *no single stolen artifact* — disk, backup, config file, one human's share — suffices to decrypt anything.
- **Example:** OpenBao with 5 Shamir shares, threshold 3, held by different people: server restarts sealed (all secrets are ciphertext on disk); three keyholders each submit their share; the root key exists in memory only; the service unseals. A stolen storage backend snapshot is useless without a quorum-level conspiracy.

### Key rotation and versioning

- **What it is:** Retiring old key material on schedule or on suspicion. With envelopes it's cheap by design: a new KEK version encrypts *newly wrapped* DEKs; old versions remain **decrypt-only** for existing wrapped DEKs (never deleted until everything under them is re-wrapped); background **re-wrap** re-encrypts the 32-byte DEKs, never the data. Every ciphertext carries its key version so decryption knows which to ask for.
- **Why it exists:** Limits how much any single key protects (blast radius over time), satisfies compliance clocks (PCI-DSS), and makes *compromise response* a routine operation instead of a crisis: suspect the KEK → rotate + re-wrap in hours, no data touched.
- **Example:** OpenBao's transit engine: `bao write -f transit/keys/orders/rotate` bumps `orders` to v4; ciphertexts are prefixed `vault:v3:...` / `bao:v4:...`; old versions decrypt until you raise `min_decryption_version` — the ratchet that eventually forces re-wrap completion.

### Crypto-shredding

- **What it is:** Deletion by key destruction: give each user (or record, or tenant) their own DEK; to erase, destroy that DEK — every copy of the data, including immutable logs and offline backups, becomes permanently unreadable at once.
- **Why it exists:** Real deletion of replicated, backed-up, event-sourced data is somewhere between hard and impossible — the copies are the *point* of those systems. Shredding inverts the problem: don't chase the data, kill the one small thing it all depends on. This is the standard answer to GDPR erasure in append-only stores (the [event sourcing GDPR pitfall](../event-sourcing/learning.md)).
- **Example:** `UserRegistered{name_enc, email_enc}` events encrypted with user 42's DEK, wrapped and stored in a keyring table (or as an OpenBao transit key per user). Erasure request → destroy key 42 → every event, projection copy, and backup tape holding that ciphertext is noise. The keyring is now the crown jewel: it must be backed up (losing it = losing all users' data) yet its deletions must be real and audited (that *is* the erasure).

### Secrets management (the adjacent, often-confused discipline)

- **What it is:** Managing *credentials* — database passwords, API tokens, TLS private keys: storing them encrypted, controlling access by identity and policy, auditing every read, rotating them, and ideally replacing them with **dynamic secrets** (short-lived credentials minted on demand, auto-revoked at TTL). This is OpenBao/Vault's home turf.
- **Why it exists:** The most common real-world key/secret failure isn't broken crypto — it's a credential in a `.env` file, a repo, a CI log. Centralizing secrets behind identity + policy + audit turns "who knows the DB password" from folklore into a queryable fact; dynamic secrets go further and make the answer "nobody, for longer than 15 minutes."
- **Example:** Instead of a shared Postgres password in twelve deploy configs: each service authenticates to OpenBao (Kubernetes/AppRole identity) → policy grants `database/creds/orders-ro` → OpenBao *creates a fresh Postgres role* valid 15 min, auto-renewed while the service lives, dropped at TTL. A leaked credential expires before the attacker finishes reading it. Encryption keys and secrets meet in the **transit engine**: OpenBao holds the KEK as a "secret" and performs encrypt/decrypt/rewrap as an API — encryption-as-a-service, the envelope pattern with the KMS role played by software you run.

## Worked Example

Requirement: user PII (email, phone) in Postgres must survive a stolen database *and* support GDPR erasure. Full-disk encryption is already on — and irrelevant to both threats once the app or the DB credentials are compromised. We build application-layer envelope encryption with OpenBao's transit engine as the KMS.

**1. Stand up the key infrastructure.**

```
bao secrets enable transit
bao write -f transit/keys/pii-kek                 # the KEK: created inside OpenBao, never exportable
bao policy write orders-svc - <<EOF
path "transit/encrypt/pii-kek"  { capabilities = ["update"] }
path "transit/decrypt/pii-kek"  { capabilities = ["update"] }
EOF                                               # note: no read on the key itself — it can't leave
```

The service can *use* the KEK; no one can *fetch* it. That asymmetry is the point of the whole architecture.

**2. Write path (per user, first touch): mint and wrap a DEK.**

```
app:  dek = random_bytes(32)                                      # generated locally
app:  ct_email = AES_256_GCM(dek, nonce1, "ana@example.com", aad="user:42:email")
app:  ct_phone = AES_256_GCM(dek, nonce2, "+31-6-...",        aad="user:42:phone")
app → bao:  POST transit/encrypt/pii-kek  {plaintext: base64(dek)}
bao → app:  "bao:v1:8f3a..."                                      # the wrapped DEK
app:  INSERT INTO user_keys(user_id, wrapped_dek) VALUES (42, 'bao:v1:8f3a...')
      INSERT INTO users(id, email_ct, phone_ct, ...) VALUES (42, ct_email, ct_phone, ...)
app:  zeroize(dek)                                                # gone from memory
```

OpenBao saw 32 bytes, never the email. Postgres holds ciphertext and a wrapped (inert) DEK. The stolen-database attacker holds nothing.

**3. Read path.**

```
app:  SELECT wrapped_dek FROM user_keys WHERE user_id = 42
app → bao:  POST transit/decrypt/pii-kek  {ciphertext: "bao:v1:8f3a..."}
bao → app:  base64(dek)                        # after checking policy; after writing an audit line
app:  email = AES_256_GCM_open(dek, ct_email, aad="user:42:email")
app:  cache dek (memory only, short TTL)       # amortize the round-trip across this user's fields
```

One small KMS call per user per cache-lifetime — not per field, not per gigabyte. The audit log now contains *every key access by identity*, which is the compliance story auditors actually accept.

**4. Rotation day (or breach-suspicion day).**

```
bao write -f transit/keys/pii-kek/rotate                       # KEK v2; v1 becomes decrypt-only
background job:  for each user_keys row:
                   POST transit/rewrap/pii-kek {ciphertext: wrapped_dek}   # v1-wrapped → v2-wrapped
                   UPDATE user_keys SET wrapped_dek = <new>
bao write transit/keys/pii-kek/config min_decryption_version=2   # after the job completes: v1 dead
```

Rows touched: one per *user*. Data re-encrypted: zero bytes. The `rewrap` endpoint never returns the DEK — OpenBao unwraps-and-rewraps internally, so even the rotation job can't see key material.

**5. Erasure request for user 42.**

```
DELETE FROM user_keys WHERE user_id = 42        -- audited, tombstoned
```

`email_ct` in the table, in last month's backups, in the read replica, in the event stream — all simultaneously noise. Nothing else changes; nothing else needs to. (If per-user keys were transit keys instead of wrapped DEKs, this would be `bao delete transit/keys/user-42` — same idea, heavier per-user cost; the wrapped-DEK keyring is the usual shape at scale.)

**6. What each attacker now gets.** Stolen disk/backup: ciphertext + wrapped DEKs — nothing. Stolen DB credentials: same. Compromised app instance: the PII *it actively touches* while compromised (app-layer encryption's honest limit — it shrinks the window, not to zero) and every access is in the audit log. Compromised OpenBao alone: KEKs but no data. Only app + OpenBao together fall over completely — and that combination is what your monitoring is for.

## Pitfalls in Depth

### Pitfall: Nonce reuse under one key (the GCM catastrophe)

- **What goes wrong:** Two messages encrypted with the same key *and* same nonce under AES-GCM don't just weaken — the XOR of plaintexts leaks immediately, and the authentication key itself can be recovered, letting the attacker *forge* valid ciphertexts. Same class of failure for ChaCha20-Poly1305. This is the sharpest cliff in practical cryptography: everything is perfect until one repeat, then confidentiality *and* integrity fail together.
- **Why it happens (the mechanism):** GCM's nonce is 96 bits; random generation is safe only to ~2³² messages per key (birthday bound). Counter-based nonces are safe *per process* — then horizontal scaling duplicates the counter across instances, or a restart resets it. The failure is a systems failure (state management across processes), not a math failure, which is why it bites production and not tests.
- **How to handle it in production, and why that works:** Structure over discipline: **fresh DEK per object** (the envelope default) makes the per-key message count 1-to-few — the nonce space can't be exhausted because it's barely used. Where keys must span many messages: XChaCha20-Poly1305 (192-bit nonce — random is safe indefinitely) or AES-GCM-SIV (nonce *misuse-resistant* — a repeat degrades gracefully instead of catastrophically). Never derive nonces from timestamps or row ids across writers.
- **Trade-offs of the fix:** Per-object DEKs mean keyring bookkeeping (you have it anyway for shredding). SIV modes cost ~2× throughput — irrelevant for PII fields, real for bulk streams. The rule that survives contact with production: *pick the construction that makes the mistake impossible, not the one that makes it avoidable.*

### Pitfall: The key next to the lock

- **What goes wrong:** Encryption is implemented; the key lives in an environment variable, a config file, the same database, or the source repo. The breach that steals the data steals the key in the same motion — the encryption was load-bearing in the compliance document and decorative in reality. Variant: full-disk/TDE encryption cited as protection against application-level compromise (it only defends stolen *hardware*; every process on the live system reads plaintext).
- **Why it happens (the mechanism):** Key delivery is the unglamorous last mile — the crypto is done, the deadline is near, and `ENCRYPTION_KEY=` in the deploy config works. Threat-model mismatch does the rest: "we encrypt at rest" satisfies the checkbox without anyone asking *against whom*.
- **How to handle it in production, and why that works:** The key must live in a **different trust domain** than the data: KMS/OpenBao (identity-gated, audited, rate-limitable), reached by *runtime authentication* (Kubernetes service account, cloud instance identity, AppRole), never by a static credential in config — otherwise the static credential is just the key wearing a hat. Write the threat model down: disk-level encryption for stolen hardware; application-layer envelopes for stolen data/credentials; nothing at rest helps against a live compromised app.
- **Trade-offs of the fix:** A hard runtime dependency on the KMS (see the availability pitfall) and real bootstrap complexity (the service must prove *its* identity first — secure introduction is genuinely irreducible). The failure mode of skipping it is silent: everything works identically until the breach, which is exactly why it gets skipped.

### Pitfall: Rolling your own crypto (one level down from where you think)

- **What goes wrong:** Nobody reimplements AES; teams instead hand-assemble *protocols* from correct primitives — encrypt-then-MAC ordered wrong, a homemade key-derivation from a password, IVs from `rand()` seeded with time, comparing MACs with `==` (timing leak), compressing before encrypting (CRIME-class length leaks). Each primitive is fine; the composition is the vulnerability.
- **Why it happens (the mechanism):** The dangerous knowledge gap is invisible: primitives compose in ways that look obviously fine and fail against attacks you've never heard of (padding oracles, nonce-truncation, canonicalization). Test suites can't catch it — the code round-trips perfectly; only an adversary exercises the flaw.
- **How to handle it in production, and why that works:** Use **misuse-resistant, joined-up APIs** that own the whole envelope: in Rust, `ring`/`aws-lc-rs` or the RustCrypto AEAD crates for primitives, `age` for file encryption, and Tink-style keyset APIs or the KMS/transit pattern (OpenBao does the composing server-side) for envelopes. Passwords never *are* keys: Argon2id to derive them. Reserve review bandwidth for the one layer you can't outsource — *which* key encrypts *what*, for *whom* — and buy everything below it.
- **Trade-offs of the fix:** Library choices constrain algorithm agility and you inherit their opinions (usually a feature). The transit pattern adds a network hop per envelope operation. Both costs are rounding errors against a padding-oracle disclosure.

### Pitfall: Rotation and shredding theater

- **What goes wrong:** Two mirror-image failures. *Rotation theater:* the KEK is "rotated" but old versions keep decrypting forever and no re-wrap job exists — new label, same exposure; or the opposite, the old version is destroyed *before* re-wrap completes and data under it is bricked. *Shredding theater:* the per-user key is "destroyed" but lives on in KMS soft-delete, a keyring backup, a replica, or a memory dump — the GDPR erasure certificate is signed while the key still exists somewhere.
- **Why it happens (the mechanism):** Both mechanisms are *processes wearing a feature's costume*. The rotate API call is instant; the actual security effect (old material retired, everything re-wrapped, ratchet advanced) is a background job with completion tracking that nobody built. Key destruction is instant; the actual erasure effect depends on every replica and backup of the *keyring* honoring it — a data-lifecycle problem, not an API call.
- **How to handle it in production, and why that works:** Make completion first-class: rotation = rotate → re-wrap job with progress metric → raise `min_decryption_version` *only at 100%* — the ratchet turns theater into enforcement (old ciphertext failing loudly beats old ciphertext quietly decryptable). Shredding = design the keyring's own lifecycle up front: where is it backed up, what's the backup retention, does destruction propagate (or do backups age out within the legally-tolerated window — document that window); disable or bound KMS soft-delete for shred keys; audit-log the destruction as the compliance artifact.
- **Trade-offs of the fix:** The ratchet means a missed row during re-wrap becomes an outage (loud failure is the feature — add a verify pass before raising the floor). Real shredding vs. keyring durability is a genuine tension: aggressive keyring backups protect against losing *everyone's* data and undermine erasure; the resolution is short, explicit backup-retention for the keyring, written into the erasure SLA.

### Pitfall: The KMS becomes the availability and latency chokepoint

- **What goes wrong:** Every decrypt in the hot path calls OpenBao. Then: OpenBao restarts *sealed* (that's the design) at 3 a.m. and every service that needs a decrypt is down until quorum unseals it; or p99 latency inherits the KMS round-trip per field; or a batch job decrypting 10M rows becomes a self-inflicted DoS on the transit endpoint; or the KMS rate-limits and the app has no fallback but 500.
- **Why it happens (the mechanism):** Centralizing keys centralizes fate — that's the deal, and it's usually signed without reading. The envelope pattern *contains* the escape hatch (DEKs are locally cacheable; the KMS is only needed to unwrap, not per-operation), but naive implementations call the KMS per read anyway, importing its availability and latency into every request.
- **How to handle it in production, and why that works:** Lean on the envelope: **cache unwrapped DEKs** in memory with bounded TTL — steady-state reads never touch the KMS; a KMS outage degrades new-key operations only, and already-cached traffic flows. Batch endpoints (transit supports batched encrypt/decrypt) for bulk jobs. For the sealed-restart problem: **auto-unseal** (root key in cloud KMS — trades the 3 a.m. quorum call for a cloud dependency) plus HA replicas so one sealed node isn't an outage. Define the degraded mode consciously: reads-of-cached-keys work, first-touch of new users fails — is that acceptable for your product? Decide before the outage does.
- **Trade-offs of the fix:** Cached DEKs widen the compromised-host window (bound the TTL; zeroize on eviction) — the classic security/availability dial. Auto-unseal moves the root of trust to the cloud provider; Shamir keeps it human and keeps the 3 a.m. call. There is no fourth option; pick with eyes open.

## Design Decisions & Trade-offs

**Threat model first, mechanism second.** Stolen hardware → disk/volume encryption (free, transparent, done). Stolen database, leaked backup, over-broad DB credentials → application-layer envelope encryption of the sensitive fields. Erasure obligations in replicated/immutable stores → per-subject DEKs + shredding. Compromised application host → *no at-rest scheme helps*; that's network segmentation, short-lived credentials, and audit. Write the sentence "this encryption defends against X and explicitly not against Y" into the design doc — it's the sentence that prevents both over- and under-engineering.

**Where the KMS role runs.** Cloud KMS (AWS/GCP/Azure): near-zero ops, HSM-rooted, per-call pricing, provider trust baked in. Self-hosted OpenBao: full control, no per-call cost, identity/policy/audit and dynamic secrets in the same tool — but now you run a tier-0 service (see the [OpenBao notes](../../oss-tools/openbao/learning.md) for the operational reality). Common hybrid: OpenBao as the application-facing KMS, auto-unsealed by a cloud KMS — cloud trust for the root, self-hosted control for everything above it.

**Key granularity is a product decision disguised as a technical one.** Per-field keys: maximal control, maximal bookkeeping. Per-user: the natural unit when erasure is the driver (one user = one shred). Per-tenant: the natural unit for B2B isolation stories ("your key, destroyable on contract exit"). Per-table/service: cheap, but shredding and blast-radius stories collapse to all-or-nothing. Default that serves most systems: per-user DEKs for PII, per-service KEKs in the KMS, coarse keys for non-subject data.

**Encrypt-locally vs. transit-encrypts.** Sending plaintext to OpenBao's transit to encrypt (it does the AEAD) is simpler — no client-side crypto at all — but ships your data through the KMS and spends its throughput. Local AEAD with transit only wrapping DEKs keeps data local and the KMS load tiny (the worked example's shape). Rule: transit-encrypts for low-volume secrets and tokens; local-with-wrapped-DEKs for anything measured in rows or gigabytes.

**Searchability, the honest version.** Encrypted fields can't be `WHERE email = ?`'d. The workable answer is **blind indexing** (a keyed HMAC of the normalized value in a separate indexed column — equality lookups only, and the index key is itself KMS-managed); deterministic encryption gives the same and leaks equality patterns more broadly. Range/like/full-text over encrypted data means either searchable-encryption research territory or admitting that field shouldn't be encrypted at this layer. Deciding *which queries the ciphertext must support* belongs in the same design meeting as the threat model.

**In Rust specifically:** `aws-lc-rs`/`ring` or RustCrypto's `aes-gcm`/`chacha20poly1305` for AEAD, `argon2` for password-derived keys, `zeroize` for scrubbing DEKs after use, `secrecy` types to keep keys out of `Debug`/logs by construction, and the OpenBao/Vault HTTP API is plain enough that `reqwest` + a thin typed client suffices. The type system is genuinely useful here: a `SecretBox<Dek>` that can't be logged or cloned casually is the kind of misuse-resistance this domain rewards.

## Open Questions

- Blind indexes in practice: normalization pitfalls (case, unicode), index-key rotation story, and how much equality-pattern leakage matters per field type — work a real schema.
- AES-GCM-SIV vs XChaCha20-Poly1305 as the "safe default" for multi-message keys: benchmark both in Rust (`aes-gcm-siv` vs `chacha20poly1305` crates) and check hardware support on our targets.
- Transit DEK caching: what TTL/eviction policy balances the compromised-host window against KMS availability — and does a `zeroize`-on-evict LRU exist off the shelf, or is it a build?
- The keyring backup vs. erasure tension: what do teams actually write in their GDPR erasure SLA about backup retention windows? Find three real policies.
- Deterministic encryption vs. blind index for the email-uniqueness check (ties into the event-sourcing set-validation problem — the reservation stream would hold a blind index value, not plaintext).
- Post-quantum: for *at-rest* envelopes the harvest-now-decrypt-later risk applies to the KEK wrapping — when do KMS/OpenBao grow ML-KEM hybrid wrapping, and does anything need doing before that?

## References

- *Cryptography Engineering* (Ferguson, Schneier, Kohno) — the design-level book: why protocols fail, what the primitives assume; the right depth for architects (skip the math-heavy alternatives first pass).
- AWS KMS Developer Guide, ["Envelope encryption" concepts chapter](https://docs.aws.amazon.com/kms/latest/developerguide/concepts.html) — the cleanest short statement of the DEK/KEK pattern, vendor specifics easily generalized.
- [OpenBao transit secrets engine docs](https://openbao.org/docs/secrets/transit/) — the concrete API for everything in the worked example: rotate, rewrap, min_decryption_version, batching.
- Google Tink's ["I want to..." docs](https://developers.google.com/tink) — worth reading even from Rust for the misuse-resistant API philosophy: keysets, versioning, and AEAD-with-associated-data as the default envelope.
- Latacora, ["Cryptographic Right Answers"](https://latacora.micro.blog/2018/04/03/cryptographic-right-answers.html) — the opinionated cheat sheet for primitive choices; check for the updated edition.
- Related topics in this repo: [Event Sourcing & CQRS](../event-sourcing/learning.md) (crypto-shredding is its GDPR answer), [OpenBao](../../oss-tools/openbao/learning.md) (the tool that plays the KMS role here), [Idempotency & Delivery Semantics](../idempotency-and-delivery-semantics/learning.md) (the keyring-deletion audit trail is another at-least-once effect chain).
