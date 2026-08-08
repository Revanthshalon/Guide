# Encryption & Key Management — Quick Reference

Core model: encryption exchanges a big secret (data) for a small one (the key); key management is the recursion "what protects the key?" — DEK wraps data, KEK wraps DEK (envelope), hierarchy terminates at a root of trust (HSM / Shamir quorum / cloud KMS). Details in [learning.md](learning.md).

## Which Mechanism for Which Attacker

| Threat | Mechanism | Note |
| --- | --- | --- |
| Stolen disk/hardware | Full-disk / volume encryption | Free, transparent — and defends *only* this |
| Stolen DB, leaked backup, over-broad DB creds | Application-layer envelope encryption per field | DEK local, KEK in KMS/OpenBao |
| Erasure in replicated/immutable stores (GDPR) | Per-subject DEK + crypto-shredding | Destroy key = all copies noise |
| Compromised live application | Nothing at-rest helps | Segmentation, short-lived creds, audit |
| Untrusted network | TLS everywhere | Different axis; don't conflate |

## Primitive Choices (don't improvise)

| Need | Use | Never |
| --- | --- | --- |
| Bulk/field encryption | AEAD: AES-256-GCM or ChaCha20-Poly1305, with AAD binding context | Unauthenticated modes (CBC without MAC) |
| One key, many messages | XChaCha20-Poly1305 (192-bit nonce) or AES-GCM-SIV | Counter nonces across processes; random 96-bit past ~2³² msgs |
| Key from password | Argon2id | Password as key; fast hashes |
| Rust | `aws-lc-rs`/`ring`, RustCrypto AEADs, `zeroize`, `secrecy` | Hand-composed protocols |

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| Nonce reuse under one GCM key | Fresh DEK per object (envelope default); SIV/XChaCha for multi-message keys | Catastrophic: leaks plaintext AND forgery ability |
| Key stored next to data (env var, config, same DB) | Key in a different trust domain; runtime identity auth to KMS, no static creds | Static KMS credential = the key wearing a hat |
| Hand-rolled protocol composition | Misuse-resistant APIs; transit/KMS does the envelope server-side | Primitives fine, composition fatal; tests can't catch it |
| Rotation theater | Rotate → tracked re-wrap job → raise `min_decryption_version` at 100% | Old versions decrypting forever; or bricking data by destroying early |
| Shredding theater | Keyring lifecycle designed: bounded backups, soft-delete bounded, destruction audited | Key "destroyed" but alive in KMS soft-delete / keyring backup |
| KMS as availability/latency chokepoint | Cache unwrapped DEKs (TTL + zeroize); batch endpoints; auto-unseal + HA | Cached DEKs widen compromised-host window — bound TTL |

## Production Checklist

- [ ] Threat-model sentence written: defends against X, explicitly not Y
- [ ] AEAD everywhere, AAD binds ciphertext to its record/context
- [ ] DEK per user/object; KEK never leaves KMS/OpenBao (no read capability on key)
- [ ] Ciphertext carries key version; re-wrap job + ratchet tested
- [ ] Keyring backup retention written into erasure SLA
- [ ] DEK cache: bounded TTL, zeroize on evict; degraded mode defined for KMS outage
- [ ] Unseal story decided (Shamir quorum vs auto-unseal) and rehearsed
- [ ] Every key access audited by identity; searchable fields use blind indexes deliberately

## Key References

- AWS KMS Developer Guide, envelope encryption concepts.
- [OpenBao transit docs](https://openbao.org/docs/secrets/transit/) — rotate/rewrap/batch APIs.
- Latacora, "Cryptographic Right Answers."
