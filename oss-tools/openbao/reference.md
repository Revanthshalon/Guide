# OpenBao — Quick Reference

Core model: self-hosted secrets management + encryption-as-a-service; the MPL-licensed Linux Foundation fork of Vault 1.14 after HashiCorp's BUSL relicense. Vault's mental model, `bao` CLI, compatible core API. Details in [learning.md](learning.md).

## Quick Facts

- **Alternative to:** HashiCorp Vault
- **License:** MPL 2.0 (Vault is BUSL 1.1 since Aug 2023)
- **Backed by:** Linux Foundation, open governance
- **Storage:** Integrated Raft (3/5 nodes) — it's a consensus system; treat as tier-0
- **Sibling fork:** OpenTofu (Terraform, same licensing event)

## Comparison

| Aspect | Vault | OpenBao |
| --- | --- | --- |
| Core workflow (KV, transit, DB creds, PKI, auth, policies) | ✓ | ✓ drop-in for most |
| Namespaces | Enterprise | Open version |
| Post-fork features (Vault 1.15+) | ✓ | Only if reimplemented — check per feature |
| Legacy storage backends / niche plugins | Long list | Trimmed at fork — check inventory |
| Enterprise replication/DR, Sentinel | Paid | Absent / community equivalents vary |
| Ecosystem integrations | Deepest | Rides compatibility; verify per vendor tool |

## Common Commands

```sh
bao server -dev                                   # local exploration only
bao status                                        # sealed? leader? version?
bao operator unseal / bao operator raft snapshot save backup.snap
bao secrets enable -path=secret kv-v2
bao kv put secret/app/db password=...
bao secrets enable transit
bao write -f transit/keys/my-kek                  # create KEK (non-exportable)
bao write transit/encrypt/my-kek plaintext=$(base64)   # wrap a DEK
bao write transit/rewrap/my-kek ciphertext=...    # rotate without exposing DEK
bao write transit/keys/my-kek/config min_decryption_version=N   # the ratchet
bao policy write name file.hcl
bao lease revoke -prefix database/creds/          # incident mass-revocation
bao operator generate-root                        # break-glass via unseal quorum
```

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| Root token kept "for emergencies" | Revoke after setup; break-glass = `generate-root` via unseal quorum, rehearsed | Root use is invisible in policy terms |
| Run like a stateless app | Tier-0 ops: 3/5-node Raft, off-cluster restore-tested snapshots, seal/quorum monitoring | Untested backup of an encrypted store = hope |
| Static secrets moved, static habits kept | Platform-identity auth first, then dynamic engines by blast radius; metric: % creds expiring ≤1 h | Long-lived token fetching secrets = same problem, indirected |
| Lease explosion / revocation storms | TTL ≈ process lifetime; one credential per workload (agent-held); lease-count monitoring per mount | Crash loops mint credentials silently |
| Vault migration assumed compatible | Inventory every mount/plugin/client vs. support matrix; parallel-run, migrate per mount | Transit keys: expect re-wrap, not export; post-1.14 features |
| Audit device fills disk | Size + monitor; all-devices-unwritable blocks requests | By design, not a bug |

## Migration Checklist (from Vault)

- [ ] Inventory: mounts, auth methods, plugins, policy features, all client integrations
- [ ] Risk list: Enterprise-only + post-1.14 features, each with a plan
- [ ] Fresh OpenBao cluster (Raft, unseal, audit, snapshots) — migrate via API per engine, not data-dir copy
- [ ] Machine-identity auth + policies (from code) first; canary service
- [ ] Mounts incrementally: KV → dynamic DB → PKI/transit last (longest-lived artifacts)
- [ ] Transit: recreate keys, re-wrap ciphertexts, version-ratchet cutover
- [ ] CI/agents: `vault`→`bao`, `VAULT_ADDR`→`BAO_ADDR`, pinned client libs tested
- [ ] Vault read-only one full lease-lifetime + rotation cycle, then decommission

## Key References

- [OpenBao docs](https://openbao.org/docs/) — source of truth for current feature state.
- [github.com/openbao/openbao](https://github.com/openbao/openbao) — changelog = the divergence record.
- [Encryption & Key Management](../../architecture-patterns/encryption-and-key-management/learning.md) — the envelope pattern transit implements.
