# Graph Report - system-architecture  (2026-08-15)

## Corpus Check
- 198 files · ~379,016 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 1205 nodes · 1424 edges · 133 communities (102 shown, 31 thin omitted)
- Extraction: 74% EXTRACTED · 25% INFERRED · 1% AMBIGUOUS · INFERRED: 359 edges (avg confidence: 0.84)
- Token cost: 1,543,836 input · 0 output

## Community Hubs (Navigation)
- Compiler & Data-Oriented Optimization
- Traversal and Hash Table Internals
- Delivery Semantics and Idempotency
- Balanced Trees and Bit Tricks
- Binary Search and Layout
- Sorting and External Merge
- False Sharing and Coherence
- Stacks, Queues, Ring Buffers
- Graph Representations (CSR)
- Rust Benchmarking Hygiene
- Monotonic Structures and Prefix Sums
- Regex Search Tooling
- Overload Defense and Rate Limiting
- Git Undo and Recovery
- Persistent Immutable Structures
- Backpressure and Queueing
- B-Trees and Low-Link Origins
- Randomization and Backtracking
- Heaps and Approximation Ratios
- Modular Arithmetic and Combinatorics
- PostgreSQL Storage and Planner
- Branch Prediction and Branchless Code
- Performance Doc Templates
- Encryption and Key Hierarchy
- Secret Engines and State Secrets
- Quorums and Broker Semantics
- Knowledge Base Structure
- Flow, Matching, and Reductions
- Intervals, MST, and Intractability
- Primality, Las Vegas, Selection
- Async Runtimes and Allocators
- Event Sourcing Pitfalls
- Change Data Capture
- Events vs Commands (EDA)
- Strangler Fig Migration
- BFS Variants and NP-Completeness
- Nearest Neighbour and kd-trees
- Docker Build Caching and Release
- Allocation Levers and Batching
- Consistent Hashing and Shard Skew
- Load Balancing and Vec Growth
- Outbox Relay and Replication Lag
- Greedy Proofs and MST Properties
- Sketches and Probabilistic Filters
- Content-Addressed Object Models
- Container Isolation and Hardening
- Machine Identity and CI Credentials
- Raft Storage, Seal, and Backup
- Shard Keys and SCC Algorithms
- BTreeMap Fanout and Crossovers
- Prim's Algorithm and Indexed Heaps
- Greedy Scheduling and Sweep Status
- Streaming Sketches and FM-Index
- Unicode Levels and Text Ranges
- Leases, Audit, and Root Tokens
- Terraform State Drift Model
- Cache Invalidation and Idempotency Keys
- Breaker Placement and Deferred Topics
- Kruskal, DSU, and Edge Lists
- Sweep Line and Bucket Queues
- Mergeable Quantiles and Top-k
- Type-Driven State Modelling
- Consensus, Leases, Fencing
- The Vec Invariant
- Hash Flooding and Randomized State
- Heap Property and Heapify
- Rust Ownership Practices
- Error Design and Test Coverage
- Container State and Volume Backups
- Connection Pooling and C10K
- Cache Lines and Working Sets
- Deadlines and Load Shedding
- Cache-Aside and Snapshot Bootstrap
- Power of Two Choices
- Quicksort and pdqsort Lineage
- Rebase vs Merge Discipline
- Secret Rotation and Worktrees
- ripgrep Skipping and -u Ladder
- Health Checks and Outlier Ejection
- LCA and Heavy-Light Decomposition
- Bit Idioms and Submask Enumeration
- Asymptotics and Accidental Quadratics
- Heavy Hitters and TinyLFU
- SIMD Substring Search
- Percentiles and CI Noise Floors
- OCI Lineage and Licensing
- Provider Pinning and Upgrades
- State File Layout and Surgery
- Schema Evolution Coupling
- Finishing the Migration
- Memory Reclamation and ABA
- Hash/Eq Contract
- Unicode Normalization and Collation
- Suffix Array Construction (SA-IS)
- Sliding Window Preconditions
- Git Three Areas and Worktrees
- Searching Git History
- sed Substitution Hazards
- sed Addressing Model
- Test Isolation and Determinism
- Vault to OpenBao Migration
- TLB, Cache, Branch Signatures
- Cache Placement and Eviction
- CRDTs vs Last-Writer-Wins
- Split Brain and RPO
- Vec Removal Costs
- Linearizability Foundations
- Memory Ordering and Verification
- Progress Guarantees
- Adjacency List Invariant
- Hash Avalanche and Modulo Bias
- Interval Trees vs Offline Sweep
- LinkedList Splice Trade-off
- Shuffling and Range Bias
- Grids, Quadtrees, Octrees
- R-trees and Bounding Rectangles
- Queue Policy and Little's Law
- Prefix Doubling Suffix Arrays
- Cherry-Picking Reference
- Module Addressing and moved
- Online Schema Migrations
- petgraph Crate
- Pairing Heap
- FFT / NTT Convolution
- sort_by_cached_key
- Spatial Data Structures Doc
- Stacks & Queues Doc
- Streaming Algorithms Doc
- Boyer-Moore
- Case Conversion Is Not Per-Character
- Two Pointers Doc
- Merge Conflict Config (zdiff3, rerere)
- First-Touch Page Faults

## God Nodes (most connected - your core abstractions)
1. `Profiling and Measurement — Learning` - 18 edges
2. `Memory Layout — Learning` - 17 edges
3. `Data-Oriented Design — Learning` - 16 edges
4. `Parallelism and Work Stealing — Learning` - 16 edges
5. `Serialization and Encoding — Learning` - 16 edges
6. `SIMD — Learning` - 16 edges
7. `Compiler Optimizations — Learning` - 15 edges
8. `False Sharing — Learning` - 14 edges
9. `Lock-Free Concurrency — Learning` - 14 edges
10. `Zero-Copy — Learning` - 13 edges

## Surprising Connections (you probably didn't know these)
- `Copy-on-Write B-Trees (Btrfs, LMDB — Root Swap as Atomic Commit)` --semantically_similar_to--> `Linearizability`  [INFERRED] [semantically similar]
  data-structures-and-algorithms/b-trees/learning.md → architecture-patterns/replication-and-consistency/learning.md
- `Pitfall: Secrets Baked Into Image Layers` --semantically_similar_to--> `Committed Secret: Rotate First`  [INFERRED] [semantically similar]
  oss-tools/docker/learning.md → developer-tooling/git/recipes.md
- `Skip Files Entirely (ripgrep's Central Design Decision)` --semantically_similar_to--> `The Build Cache Is Order-Dependent`  [INFERRED] [semantically similar]
  developer-tooling/ripgrep-and-grep/learning.md → oss-tools/docker/learning.md
- `async fn as a State-Machine Enum` --semantically_similar_to--> `Process-Per-Connection Model`  [INFERRED] [semantically similar]
  performance-optimization/async-and-io/learning.md → oss-tools/postgres/learning.md
- `Branches Aren't Slow — Surprising Branches Are Slow` --semantically_similar_to--> `Cost-Based, Statistics-Driven Planner`  [INFERRED] [semantically similar]
  performance-optimization/branch-prediction/learning.md → oss-tools/postgres/learning.md

## Import Cycles
- None detected.

## Hyperedges (group relationships)
- **Resilience Family: Defending Against Overload** — architecture_patterns_circuit_breaker_learning_state_machine, architecture_patterns_circuit_breaker_learning_bulkheads, architecture_patterns_backpressure_and_rate_limiting_learning_concurrency_limits, architecture_patterns_backpressure_and_rate_limiting_learning_load_shedding, architecture_patterns_caching_strategies_learning_cache_aside, architecture_patterns_backpressure_and_rate_limiting_learning_deadline_propagation [EXTRACTED 1.00]
- **At-Least-Once Guarantee: Producer and Consumer Halves** — architecture_patterns_learning_index_outbox_pattern, architecture_patterns_idempotency_and_delivery_semantics_learning_idempotency_key, architecture_patterns_idempotency_and_delivery_semantics_learning_dedup_window, architecture_patterns_event_sourcing_learning_dual_write_pitfall, architecture_patterns_change_data_capture_learning_log_based_capture [INFERRED 0.85]
- **Crypto-Shredding as the GDPR Answer to Immutable Logs** — architecture_patterns_encryption_and_key_management_learning_crypto_shredding, architecture_patterns_encryption_and_key_management_learning_envelope_encryption, architecture_patterns_event_sourcing_learning_encrypted_event_payloads, architecture_patterns_event_sourcing_learning_gdpr_vs_immutability, architecture_patterns_event_sourcing_learning_event_store [EXTRACTED 1.00]
- **Reliable Cross-Service Messaging: Outbox + Idempotent Inbox + Saga Steps** — architecture_patterns_outbox_pattern_learning_outbox_table, architecture_patterns_outbox_pattern_learning_inbox_pattern, architecture_patterns_idempotency_and_delivery_semantics_reference_at_least_once, architecture_patterns_saga_pattern_learning_saga [EXTRACTED 1.00]
- **Incremental Migration: Dual-Write, Backfill, Verify, Cutover, Delete** — architecture_patterns_strangler_fig_learning_sync_direction, architecture_patterns_strangler_fig_learning_shadow_traffic, architecture_patterns_strangler_fig_learning_deletion_plan, architecture_patterns_sharding_learning_migration_sequence [INFERRED 0.85]
- **Low-Link Yields SCCs, Bridges, Articulation Points, and 2-SAT** — data_structures_and_algorithms_advanced_graph_algorithms_learning_low_link, data_structures_and_algorithms_advanced_graph_algorithms_learning_tarjan_scc, data_structures_and_algorithms_advanced_graph_algorithms_learning_two_sat, data_structures_and_algorithms_advanced_graph_algorithms_learning_condensation [EXTRACTED 1.00]
- **Constant-Factor Wins from Memory Layout (same asymptotics, different hardware fit)** — data_structures_and_algorithms_cache_aware_structures_learning_flat_beats_pointer_pattern, data_structures_and_algorithms_bit_manipulation_learning_bitset, data_structures_and_algorithms_edit_distance_and_alignment_learning_myers_bit_parallel, data_structures_and_algorithms_cache_aware_structures_learning_eytzinger_layout, data_structures_and_algorithms_binary_search_trees_reference_btreemap [INFERRED 0.85]
- **Recurrence Analysis Toolkit (Master Theorem applied across algorithms)** — data_structures_and_algorithms_complexity_analysis_learning_master_theorem, data_structures_and_algorithms_divide_and_conquer_learning_divide_and_conquer, data_structures_and_algorithms_divide_and_conquer_learning_karatsuba, data_structures_and_algorithms_divide_and_conquer_learning_strassen, data_structures_and_algorithms_binary_search_learning_binary_search [EXTRACTED 1.00]
- **Space vs Reconstruction Trade-off in Table-Based DP** — data_structures_and_algorithms_dynamic_programming_learning_rolling_array, data_structures_and_algorithms_edit_distance_and_alignment_learning_hirschberg_alignment, data_structures_and_algorithms_dynamic_programming_reference_hirschberg, data_structures_and_algorithms_edit_distance_and_alignment_learning_levenshtein [EXTRACTED 1.00]
- **The Frontier Container Determines the Algorithm (BFS/DFS/Dijkstra/Prim)** — data_structures_and_algorithms_graph_traversal_learning_unified_traversal, data_structures_and_algorithms_graph_traversal_learning_bfs, data_structures_and_algorithms_graph_traversal_learning_dfs, data_structures_and_algorithms_heaps_and_priority_queues_learning_binary_heap, data_structures_and_algorithms_minimum_spanning_trees_learning_prim_vs_dijkstra [INFERRED 0.85]
- **Flat Contiguous Arrays Beat Pointer-Linked Structures at Equal Asymptotics** — data_structures_and_algorithms_graph_representations_learning_csr, data_structures_and_algorithms_linked_lists_learning_traversal_measurement, data_structures_and_algorithms_heaps_and_priority_queues_learning_shape_property, data_structures_and_algorithms_heaps_and_priority_queues_learning_fibonacci_heap, data_structures_and_algorithms_linked_lists_learning_arena_representation [INFERRED 0.85]
- **Greedy's Proof Obligation: Cut Property → Matroid → MST → Approximation Ratios** — data_structures_and_algorithms_greedy_algorithms_learning_exchange_argument, data_structures_and_algorithms_greedy_algorithms_learning_matroid, data_structures_and_algorithms_minimum_spanning_trees_learning_cut_property, data_structures_and_algorithms_minimum_spanning_trees_learning_kruskal, data_structures_and_algorithms_intractability_and_approximation_learning_approximation_ratio [EXTRACTED 1.00]
- **Invertibility Determines Which Range Structure You Can Use** — data_structures_and_algorithms_prefix_sums_and_difference_arrays_learning_invertibility_requirement, data_structures_and_algorithms_prefix_sums_and_difference_arrays_learning_prefix_sum_array, data_structures_and_algorithms_range_query_structures_learning_fenwick_tree, data_structures_and_algorithms_range_query_structures_learning_segment_tree, data_structures_and_algorithms_range_query_structures_learning_sparse_table [EXTRACTED 1.00]
- **Error Direction + Mergeability Govern Every Sketch's Safe Use** — data_structures_and_algorithms_probabilistic_data_structures_learning_bloom_filter, data_structures_and_algorithms_probabilistic_data_structures_learning_hyperloglog, data_structures_and_algorithms_probabilistic_data_structures_learning_count_min_sketch, data_structures_and_algorithms_probabilistic_data_structures_learning_mergeability, data_structures_and_algorithms_selection_and_order_statistics_learning_never_average_percentiles [EXTRACTED 1.00]
- **The Fixed-Size Call Stack as a Cross-Topic Failure Mode** — data_structures_and_algorithms_recursion_and_backtracking_learning_stack_depth_guard, data_structures_and_algorithms_recursion_and_backtracking_learning_no_guaranteed_tco, data_structures_and_algorithms_rust_for_data_structures_learning_recursive_drop_overflow, data_structures_and_algorithms_rust_for_data_structures_learning_box_ownership_tree, data_structures_and_algorithms_persistent_immutable_structures_learning_path_copying [INFERRED 0.85]
- **Preprocess-the-Text vs Preprocess-the-Pattern Trade-off** — data_structures_and_algorithms_string_matching_learning_str_find_two_way, data_structures_and_algorithms_string_matching_learning_aho_corasick, data_structures_and_algorithms_suffix_structures_learning_suffix_array, data_structures_and_algorithms_suffix_structures_learning_preprocess_text_not_pattern, data_structures_and_algorithms_tries_and_radix_trees_learning_fst_crate [EXTRACTED 1.00]
- **Contiguity Beats Better Asymptotics (The Flat Array Keeps Winning)** — data_structures_and_algorithms_suffix_structures_learning_flat_array_keeps_winning, data_structures_and_algorithms_tries_and_radix_trees_learning_pitfall_uncompressed_trie, data_structures_and_algorithms_tries_and_radix_trees_learning_sorted_vec_partition_point, data_structures_and_algorithms_string_matching_learning_asymptotics_rank_inversely, data_structures_and_algorithms_sorting_learning_quicksort [INFERRED 0.85]
- **The "Each Element Enters and Leaves Once" Amortization Argument** — data_structures_and_algorithms_two_pointers_and_sliding_window_learning_amortization_enter_once_leave_once, data_structures_and_algorithms_string_matching_learning_failure_function, data_structures_and_algorithms_suffix_structures_learning_kasai_algorithm, data_structures_and_algorithms_stacks_and_queues_learning_ring_buffer_invariant [EXTRACTED 1.00]
- **Safe Bulk Edit Pipeline (rg → NUL → sd/sed → git diff review)** — developer_tooling_ripgrep_and_grep_recipes_safe_bulk_edit, developer_tooling_ripgrep_and_grep_learning_nul_separation, developer_tooling_ripgrep_and_grep_recipes_sd, developer_tooling_sed_and_text_processing_recipes_portable_in_place, developer_tooling_sed_and_text_processing_recipes_bulk_edit_safety_checklist, developer_tooling_git_recipes_three_dot_diff [EXTRACTED 1.00]
- **Rust Ship Discipline: Profiles, Tests, and Measurement** — language_best_practices_rust_releasing_profile_measurements, language_best_practices_rust_benchmarking_criterion, language_best_practices_rust_testing_three_test_locations, language_best_practices_rust_reference_numbers_to_remember [EXTRACTED 1.00]
- **Container Image Safety: Immutable Layers, Secrets, Non-Root, Multi-Stage** — oss_tools_docker_learning_layer_immutability, oss_tools_docker_learning_secrets_in_layers, oss_tools_docker_learning_buildkit_secret_mounts, oss_tools_docker_learning_running_as_root, oss_tools_docker_learning_multi_stage_builds, oss_tools_docker_runbook_day1_hardening [EXTRACTED 1.00]
- **OpenBao Secret Lifecycle: Identity → Policy → Engine → Lease → Audit** — oss_tools_openbao_learning_auth_methods_and_identity, oss_tools_openbao_learning_policies, oss_tools_openbao_learning_secret_engines, oss_tools_openbao_learning_leases_renewal_revocation, oss_tools_openbao_learning_audit_devices [INFERRED 0.85]
- **OpenTofu State Safety: Encryption, Locking, Layout, and Surgery** — oss_tools_opentofu_learning_state, oss_tools_opentofu_learning_state_encryption, oss_tools_opentofu_learning_backends_locking_workspaces, oss_tools_opentofu_runbook_state_layout, oss_tools_opentofu_runbook_state_surgery [INFERRED 0.85]
- **Memory-Hierarchy Optimization Stack: Line, Prefetch, Placement, Amortization** — performance_optimization_cache_locality_learning_cache_line, performance_optimization_cache_locality_learning_hardware_prefetcher, performance_optimization_allocation_strategies_learning_allocator_placement_damage, performance_optimization_allocation_strategies_learning_bump_arena, performance_optimization_batching_and_amortization_learning_amortization_formula [INFERRED 0.80]
- **The Memory-Bound Sweep Optimization Stack** — performance_optimization_memory_layout_learning_line_efficiency, performance_optimization_data_oriented_design_learning_aos_to_soa, performance_optimization_simd_learning_lane_parallelism, performance_optimization_parallelism_and_work_stealing_learning_bandwidth_wall, performance_optimization_profiling_and_measurement_learning_hardware_counters_ipc [INFERRED 0.85]
- **Coherence-Traffic Diagnosis and Fix Path** — performance_optimization_false_sharing_learning_cache_line_coherence, performance_optimization_false_sharing_learning_cache_padded, performance_optimization_false_sharing_learning_thread_local_accumulate_merge, performance_optimization_lock_free_concurrency_learning_contention_not_locks, performance_optimization_numa_awareness_learning_local_vs_remote_dram, performance_optimization_parallelism_and_work_stealing_learning_hidden_serial_fraction [INFERRED 0.85]
- **Zero-Copy Decode Pipeline Across a Boundary** — performance_optimization_serialization_and_encoding_learning_serde_borrow, performance_optimization_serialization_and_encoding_learning_access_in_place_rkyv, performance_optimization_zero_copy_learning_borrowed_views, performance_optimization_zero_copy_learning_bytes_refcounted_views, performance_optimization_zero_copy_learning_amplification_trap, performance_optimization_memory_layout_learning_repr_c_vs_repr_rust [INFERRED 0.85]

## Communities (133 total, 31 thin omitted)

### Community 0 - "Compiler & Data-Oriented Optimization"
Cohesion: 0.07
Nodes (73): Bounds-Check Elision, Compiler Optimizations — Learning, Inlining as the Gateway Optimization, The Knobs Ladder, LTO and codegen-units=1, Monomorphization vs dyn Opacity, &mut as noalias, PGO and BOLT (+65 more)

### Community 1 - "Traversal and Hash Table Internals"
Cohesion: 0.04
Nodes (49): Depth-First Search, Low-Link Value, DFS Parenthesis Structure (discovery/finish intervals), Recursive DFS Stack Overflow (~200k depth), The Unified Traversal (frontier container is the algorithm), Visited-Set Choice (Vec<bool> vs bitset vs HashSet), BFS and DFS Are One Algorithm, What Only DFS Gives You (+41 more)

### Community 2 - "Delivery Semantics and Idempotency"
Cohesion: 0.05
Nodes (46): At-Least-Once Delivery, At-Most-Once Delivery, Kleppmann, Designing Data-Intensive Applications, Dedup Key Table (Key + Effect in One Transaction), Effectively-Once (At-Least-Once + Idempotent Receiver), Helland, Life Beyond Distributed Transactions, Ledger Keyed by Business Id (Deltas vs Absolutes), Process-Then-Ack Discipline (+38 more)

### Community 3 - "Balanced Trees and Bit Tricks"
Cohesion: 0.05
Nodes (46): Arena Allocation for Trees in Rust, BST Augmentation (subtree size, max endpoint), Degenerate Tree from Ordered Input, Rotation (the one primitive), Balancing Schemes (AVL, Red-black, Treap, Splay, Scapegoat), Seidel & Aragon (1996) — Treaps, Bitmask DP, Bitset (+38 more)

### Community 4 - "Binary Search and Layout"
Cohesion: 0.05
Nodes (44): Binary Search, Binary Search on the Answer, Galloping / Exponential Search, lower_bound / partition_point, Monotone Predicate Boundary, The Off-by-One Bug Family, Bloch — Nearly All Binary Searches Are Broken, Khuong & Morin — Array Layouts for Comparison-Based Searching (+36 more)

### Community 5 - "Sorting and External Merge"
Cohesion: 0.05
Nodes (43): Aggarwal & Vitter (1988) — External Sorting in the I/O Model, Ω(n log n) Comparison Lower Bound, Counting Sort, External Merge Sort, Merge Sort, Pitfall: The Float Comparator That Panics, Pitfall: Assuming Nearly-Sorted Means Fast, LSD Radix Sort (+35 more)

### Community 6 - "False Sharing and Coherence"
Cohesion: 0.11
Nodes (41): Cache Line as Unit of Coherence, CachePadded Isolation, False Sharing — Learning, MESI Coherence Protocol, perf c2c HITM Attribution, Thread-Local Accumulate and Merge, Thread-Scaling Sweep, True Sharing (+33 more)

### Community 7 - "Stacks, Queues, Ring Buffers"
Cohesion: 0.06
Nodes (34): Banker's Queue (Two Stacks), BFS/DFS Equivalence (Container Choice Is the Algorithm), LMAX Disruptor Technical Paper, Okasaki — Purely Functional Data Structures, Pitfall: Vec::remove(0) as a Queue, Ring Buffer Invariant ((head + i) % cap), Vec as Stack, VecDeque (+26 more)

### Community 8 - "Graph Representations (CSR)"
Cohesion: 0.10
Nodes (22): Adjacency Matrix, CSR Build as Counting Sort by Source Vertex, Compressed Sparse Row (CSR), Density Rule (d = E/V²), Shun & Blelloch, Ligra (2013), Reverse CSR, Structure-of-Arrays tgt/wt Split, Vertex ID Width (u32 targets, u64 offsets) (+14 more)

### Community 9 - "Rust Benchmarking Hygiene"
Cohesion: 0.10
Nodes (22): Language Practice Doc Template, black_box — Cost and Correct Placement, criterion Harness (harness = false, baselines), iter_batched — Excluding Setup From the Measurement, The Microbenchmark Mirage (Amdahl, cache privileges), The Optimizer Deletes Your Benchmark (0.04 ns/iter tell), Always Sweep, Never One Input Size, Practice: Derive Hygiene, #[non_exhaustive], #[must_use] (+14 more)

### Community 10 - "Monotonic Structures and Prefix Sums"
Cohesion: 0.12
Nodes (21): Deque Operation Ordering (maintain back, evict front, read front), Largest Rectangle in a Histogram, Monotonic Deque (Sliding Window Max), Monotonic Stack, Next Greater Element, Store Indices, Not Values, Monotonic Stack Direction Table, 2-D Prefix Sums (Inclusion-Exclusion) (+13 more)

### Community 11 - "Regex Search Tooling"
Cohesion: 0.10
Nodes (20): ast-grep (structural search), BRE vs ERE Regex Flavour Trap, Catastrophic Backtracking / ReDoS with -P, grep Portability & Variants (GNU/BSD/ugrep/busybox), Line-Oriented Tools Don't See Multi-line Structure, This Machine: ripgrep 15.2.0, grep is ugrep 7.5.0, Regex Engine Choice (finite automaton vs PCRE2), SIMD Literal Prefilter (memchr/Teddy) (+12 more)

### Community 12 - "Overload Defense and Rate Limiting"
Cohesion: 0.12
Nodes (18): Adaptive Limits, Concurrency Limits, Rate Limiting Algorithms, Retry Amplification and Metastable Collapse, Cache Avalanche (Mass Expiry or Cache Loss), Cache Stampede (Thundering Herd), Metrics That Predict Incidents, Bulkheads (Isolation) (+10 more)

### Community 13 - "Git Undo and Recovery"
Cohesion: 0.12
Nodes (18): --force-with-lease, Oh Shit, Git!?!, Reflog Recoverability Rule, Recovery via git reflog, Revert for Shared History, Rewriting Local History, Three-Dot Diff (main...feature), Undo By Intent Table (+10 more)

### Community 14 - "Persistent Immutable Structures"
Cohesion: 0.15
Nodes (16): Arc::make_mut Copy-on-Write, CAS on the Root When Publishing a Version, Immutable Structures Are Sync For Free, Path Copying, Persistence Levels (Partial / Full / Confluent), Accidentally Retained Versions, RRB-Tree (Persistent Vector), Structural Sharing (+8 more)

### Community 15 - "Backpressure and Queueing"
Cohesion: 0.14
Nodes (15): Learning Notes Template, Quick Reference Template, Backpressure Is End-to-End Or Nothing, Bounded Queues, Little's Law, Queues Convert Overload Into Latency, Pitfall: Unbounded Queues, The Three Mechanisms (Backpressure Reference) (+7 more)

### Community 16 - "B-Trees and Low-Link Origins"
Cohesion: 0.18
Nodes (15): DSA Learning Template, DSA Reference Template, Low-Link (At Most One Back Edge), Tarjan, Depth-First Search and Linear Graph Algorithms (1972), B+ Tree (Values in Leaves, Linked Leaves), B-Tree (Search Tree Redesigned Around Blocks), B-Tree Invariant (t−1 … 2t−1 Keys, Equal Leaf Depth), Bayer & McCreight, Organization and Maintenance of Large Ordered Indices (1972) (+7 more)

### Community 17 - "Randomization and Backtracking"
Cohesion: 0.16
Nodes (15): Average-Case vs Expected-Case, CSPRNG vs Fast PRNG, Seed Reproducibility in Tests and Benchmarks, Lazy Propagation, Pruning Is the Entire Algorithm, Bitmask Search State, Make/Unmake State Restoration, Memoization Converts Backtracking into DP (+7 more)

### Community 18 - "Heaps and Approximation Ratios"
Cohesion: 0.14
Nodes (14): Greedy as an Approximation (ratio, not optimum), Greedy Approximation Guarantees Table, d-ary Heap, Fibonacci Heap (loses in practice), Larkin, Sen & Tarjan, Empirical Priority Queue Study (2014), Shape Property (complete tree → flat array, no pointers), ρ-Approximation Is a Worst-Case Promise, FPTAS / PTAS / Constant / Log / Inapproximable Hierarchy (+6 more)

### Community 19 - "Modular Arithmetic and Combinatorics"
Cohesion: 0.14
Nodes (14): Extended Euclidean Algorithm, Modular Inverse via Fermat's Little Theorem, Matrix Exponentiation for Linear Recurrences, Modular Exponentiation (modpow / exponentiation by squaring), Modular Arithmetic, nCr with Precomputed Factorials, Overflow Before the Modulus (u128 reduction), Rust % is a Remainder, Not a Modulus (+6 more)

### Community 20 - "PostgreSQL Storage and Planner"
Cohesion: 0.16
Nodes (14): The B-tree Heap/Index Split, Cost-Based, Statistics-Driven Planner, PostgreSQL Extensions, MVCC and the Visibility Map, Pitfall: Assuming an Index Will Be Used, Pitfall: Long Transactions Blocking Vacuum, PostgreSQL — Learning Notes, WAL — Write-Ahead Log (+6 more)

### Community 21 - "Branch Prediction and Branchless Code"
Cohesion: 0.15
Nodes (14): Placement Is the Allocator's Choice (Heap Scatter), Branch Prediction — Learning Notes, Branchless Selection (cmov / masks / lookup tables), cmov Turns Control Dependency Into Data Dependency, Constant-Time Cryptographic Code (Different Objective), Indirect Branch Prediction (BTB) and dyn Trait Dispatch, Misprediction Penalty (~15–20 Cycles), Return Stack Buffer (+6 more)

### Community 22 - "Performance Doc Templates"
Cohesion: 0.23
Nodes (13): Performance Technique — Learning Template, Performance Technique — Reference Template, Allocation Strategies — Learning Notes, Allocation Strategies — Quick Reference, Async & I/O — Learning Notes, Async & I/O — Quick Reference, Batching & Amortization — Learning Notes, Batching & Amortization — Quick Reference (+5 more)

### Community 23 - "Encryption and Key Hierarchy"
Cohesion: 0.17
Nodes (12): Symmetric Encryption and AEAD, Crypto-Shredding, DEK, KEK, and the Envelope, Asymmetric Encryption and Hybrid Schemes, Key Hierarchy, Root of Trust, Unsealing, Pitfall: The Key Next to the Lock, Key Rotation and Versioning, Pitfall: Nonce Reuse Under One Key (GCM) (+4 more)

### Community 24 - "Secret Engines and State Secrets"
Cohesion: 0.17
Nodes (12): KV v2 Secret Engine, Pitfall: Static Secrets In, Static Habits Kept, OpenBao Policies, Secret Engines, Transit Engine — Envelope Encryption KMS Role, Least-Privilege Service Policy, Backends, Locking, and Workspace Layout, Pitfall: Secrets in State (and in Plans) (+4 more)

### Community 25 - "Quorums and Broker Semantics"
Cohesion: 0.18
Nodes (11): A Cache Is a Priced Bet on Staleness, Membership and Reconfiguration, Quorum Intersection, Quorum Sizing & Placement, Consumer Groups, Offsets, and Lag, Dead Letter Queues and Poison Messages, Log-Based vs Queue-Based Brokers, Partitions, Ordering, and the Key Choice (+3 more)

### Community 26 - "Knowledge Base Structure"
Cohesion: 0.20
Nodes (11): architecture-patterns Category, data-structures-and-algorithms Category, developer-tooling Category, System Architecture Knowledge Base, language-best-practices Category, Learning vs Reference Doc Split, oss-tools Category, performance-optimization Category (+3 more)

### Community 27 - "Flow, Matching, and Reductions"
Cohesion: 0.18
Nodes (11): Aspvall, Plass & Tarjan (1979) — 2-SAT via SCC, Augmenting Paths (Matching and Flow Are One Algorithm), CP-Algorithms: Graphs, Dinic's Max-Flow, Hopcroft-Karp Bipartite Matching, König's Theorem, Max-Flow Min-Cut Theorem, Min-Cost Max-Flow's Pseudo-Polynomial F Blowup (+3 more)

### Community 28 - "Intervals, MST, and Intractability"
Cohesion: 0.18
Nodes (11): Total Event Ordering Including Ties, Merging Overlapping Intervals, Branch and Bound, Exact TSP by Bitmask DP (Θ(2ⁿ·n²)), The Four Responses to Intractability, ILP / SAT Solvers as the Default Answer, 2×MST Approximation for Metric TSP, MST Bottleneck / Minimax Path (+3 more)

### Community 29 - "Primality, Las Vegas, Selection"
Cohesion: 0.18
Nodes (11): Miller-Rabin Primality Test, Sieve of Eratosthenes, Las Vegas vs Monte Carlo, Error Amplification by Repetition, Ord / Hash / Eq Trait Contracts, Median-of-Medians (BFPRT), Partition Postcondition (Neither Side Is Sorted), Quickselect (+3 more)

### Community 30 - "Async Runtimes and Allocators"
Cohesion: 0.18
Nodes (11): Allocator Size Classes and Per-Thread Caches, Blocking the Runtime — The Cardinal Sin, Cancellation Safety (Drop at Any Await), Executor, Reactor, and Waker, async fn as a State-Machine Enum, Completion Model — io_uring, Poll-Time Histogram (tokio-console) as the Flamegraph of Async, Readiness Models — epoll / kqueue (+3 more)

### Community 31 - "Event Sourcing Pitfalls"
Cohesion: 0.20
Nodes (10): Pitfall: Treating the Cache as Source of Truth, Pitfall: Row-Diffs Mistaken for Domain Events, Pitfall: Losing Causality, Event, Pitfall: Event-Sourcing Everything (Complexity Tax), Event Store, Projection / Read Model, Event Sourcing Production Checklist (+2 more)

### Community 32 - "Change Data Capture"
Cohesion: 0.22
Nodes (10): The Change Event, Log-Based Capture, Replication Slots and Retention, Trigger- and Query-Based Capture, CDC Production Checklist, Pitfall: Dual Write (Append and Publish), At-Most-Once / At-Least-Once / Effectively-Once, Pitfall: Key Recorded Separately From the Effect (+2 more)

### Community 33 - "Events vs Commands (EDA)"
Cohesion: 0.22
Nodes (10): Pitfall: The Distributed Monolith (Events as RPC), Events vs Commands, The Three Patterns (Fowler), Aggregate (Consistency Boundary), Command, Internal vs Integration Events, Saga Pattern, Sharding (+2 more)

### Community 34 - "Strangler Fig Migration"
Cohesion: 0.20
Nodes (10): Logical Shards (Over-Provisioning Indirection), Dual-Write / Backfill / Verify / Cutover Migration Sequence, Feathers, Working Effectively with Legacy Code (Seams), Fowler, StranglerFigApplication, The Interception Point (Seam / Slice Zero), Newman, Monolith to Microservices, Reversibility as the Pattern's Actual Product, Shadow Traffic and Parallel Run Verification (+2 more)

### Community 35 - "BFS Variants and NP-Completeness"
Cohesion: 0.20
Nodes (10): Breadth-First Search, Bidirectional BFS, Kahn's Topological Sort, Multi-Source BFS, Paired Near-Misses (works vs fails), P / NP / NP-complete / NP-hard, Reduction Direction (A ≤ₚ B), Restoring Structure (bipartite, DAG, planar, treewidth) (+2 more)

### Community 36 - "Nearest Neighbour and kd-trees"
Cohesion: 0.20
Nodes (10): Pitfall: Sorting to Get the Top k, select_nth_unstable, Bounding-Region Hierarchy, The Curse of Dimensionality, HNSW (Approximate Nearest Neighbour), kd-tree, LSH (Locality-Sensitive Hashing), The Pruning Rule (distance_to_plane² < best²) (+2 more)

### Community 37 - "Docker Build Caching and Release"
Cohesion: 0.22
Nodes (10): Binary That Won't Run on the Deployment Target (glibc vs musl), Reproducible Release (clean tagged checkout, lockfile, pinned toolchain, embedded SHA), cargo-chef (Rust dependency-layer caching), Multi-Stage Builds and Minimal Runtime Images, The Build Cache Is Order-Dependent, The Dockerfile Shape That Caches, Deploy by Digest, Not by Tag, Do .dockerignore First (+2 more)

### Community 38 - "Allocation Levers and Batching"
Cohesion: 0.20
Nodes (10): Buffer Reuse via clear() Keeping Capacity, Four Levers: Allocate Less, Once, Together, Elsewhere, Swapping the Global Allocator (jemalloc / mimalloc), Inline Small Storage (SmallVec / ArrayVec / CompactStr), with_capacity — Allocate Once, Accidental-Allocation Lint List (Hot Loops), N × (F + m) vs. F + N × m, A Batch Is a Shared Fate (+2 more)

### Community 39 - "Consistent Hashing and Shard Skew"
Cohesion: 0.25
Nodes (9): Bounded-Load Consistent Hashing for Affinity, Consistent Hashing and Virtual Nodes, DeCandia et al., Dynamo: Amazon's Highly Available Key-value Store, The Hot Shard (Skew, Celebrities, Monotonic Keys), Karger et al., Consistent Hashing and Random Trees (STOC 1997), Partitioning Strategies (Range, Hash, Directory), Graefe, Modern B-Tree Techniques (2011), Node Occupancy on Disk (50% Trail, Bulk Loading, Fill Factor) (+1 more)

### Community 40 - "Load Balancing and Vec Growth"
Cohesion: 0.22
Nodes (9): Envoy Load Balancing Documentation, Graceful Drain (Ordered Shutdown Sequence), L4 vs L7 Balancing and the gRPC/HTTP2 Multiplexing Trap, Service Discovery Mechanisms (DNS, Registry, Platform, Mesh), Slow Start for Cold Instances, Load Balancer Diagnostic Signatures Table, Geometric Growth and Amortized Push, std RawVec::grow_amortized (the Real Growth Policy) (+1 more)

### Community 41 - "Outbox Relay and Replication Lag"
Cohesion: 0.22
Nodes (9): The `id > last_seen` Poll Skips Rows Forever, Log-Tailing Relay (Outbox via CDC / Debezium), Morling/Debezium, Reliable Microservices Data Exchange With the Outbox Pattern, Per-Aggregate Ordering and Partition Key, Polling Relay (Publisher), Decision Reads vs Display Reads (Per-Query Routing), Derived Copies Lag the Source (Unifying Frame), Replication Lag Anomalies (Read-Your-Writes, Monotonic, Causal) (+1 more)

### Community 42 - "Greedy Proofs and MST Properties"
Cohesion: 0.22
Nodes (9): Brute-Force Verification Safety Net, Coin Change Counterexample ({25,10,1} for 30), Exchange Argument, Greedy-Choice Property, Greedy Paradigm, Borůvka's Algorithm (parallelizable), Cut Property, Cycle Property (the dual) (+1 more)

### Community 43 - "Sketches and Probabilistic Filters"
Cohesion: 0.22
Nodes (9): HAMT (Hash Array Mapped Trie), Bloom Filter, Count-Min Sketch, Cuckoo Filter, Double Hashing (Kirsch-Mitzenmacher), HyperLogLog, Sketches Store a Function of the Data, Not the Data, Sqrt Decomposition (+1 more)

### Community 44 - "Content-Addressed Object Models"
Cohesion: 0.22
Nodes (9): Git Content-Addressed Object Model, Working Tree → Index → Repository, OSS Tool Quick Reference Template, BuildKit Secret and SSH Mounts, Layers Are Immutable, Additive, Content-Addressed, PID 1 Signal Semantics and Exec Form, Pitfall: Secrets Baked Into Image Layers, The Docker Model in Four Facts (+1 more)

### Community 45 - "Container Isolation and Hardening"
Cohesion: 0.22
Nodes (9): OSS Tool Runbook Template, Compose Is a Local-Dev Orchestrator, Not Production, Namespaces and cgroups — Container Is Not a VM, Podman (daemonless, rootless by default), Pitfall: Running as Root (container UID 0 is host UID 0), Compose: service_healthy, healthcheck, named volumes, Docker Security Flags (cap-drop, read-only, no-new-privileges), Docker Common Mistakes → Consequences (+1 more)

### Community 46 - "Machine Identity and CI Credentials"
Cohesion: 0.22
Nodes (9): AppRole (Machine Identity Without Platform Attestation), Auth Methods and Identity, Kubernetes Auth (Platform Identity), Response-Wrapped SecretID Delivery, Secure Introduction, Pitfall: Auto-Apply Without a Reviewed Plan, OpenTofu CI/CD Pipeline, CI Credentials via OIDC (No Stored Keys) (+1 more)

### Community 47 - "Raft Storage, Seal, and Backup"
Cohesion: 0.25
Nodes (9): Storage Backend (Integrated Raft), Pitfall: Treating OpenBao as Just-a-Database (Tier-0 Reality), Seal / Unseal and the Security Barrier, Auto-Unseal via KMS, Initialization Ceremony (Happens Exactly Once), OpenBao — Setup & Operations Runbook, Raft Snapshot Backup and Restore, Shamir Human-Quorum Unseal (+1 more)

### Community 48 - "Shard Keys and SCC Algorithms"
Cohesion: 0.25
Nodes (8): Access-Pattern Table (Choose the Key from Measured Queries), Premature Sharding (Exhaust the Cheaper Ladder First), The Shard Key (the Dominating, Irreversible Decision), SCC Condensation Yields a DAG, Write DFS Iteratively (Recursion Aborts at ~200k Depth), Kosaraju SCC, petgraph crate, Tarjan SCC

### Community 49 - "BTreeMap Fanout and Crossovers"
Cohesion: 0.25
Nodes (8): dedup Removes Only Consecutive Duplicates, Measured Crossovers (Linear vs Binary Search vs HashMap), Rust BTreeMap (B = 6, ≤11 Keys per Node), Fanout Chosen to Fill a Block, Non-Total Ord (or Mutated Key) Makes Entries Unreachable, range() — the Operation HashMap Cannot Do (~14,500×), Pitfall: Assuming a Shallower Tree Is Automatically Faster, More Comparisons, Fewer Transfers (the Correct Trade)

### Community 50 - "Prim's Algorithm and Indexed Heaps"
Cohesion: 0.25
Nodes (8): Implicit Graphs, Mark on Push vs Mark on Pop, Write Traversal Against a Neighbour Function, Indexed Heap (element → position map), Lazy Deletion (the decrease-key workaround), Lazy Prim's Θ(E) Heap, Prim's Algorithm, Prim vs Dijkstra — the One-Token Difference

### Community 51 - "Greedy Scheduling and Sweep Status"
Cohesion: 0.25
Nodes (8): Greedy Stays Ahead, Huffman Coding, Interval Scheduling (earliest finish time), Binary Heap, Streaming Top-k (size-k min-heap), Bentley-Ottmann Segment Intersection, Status-Structure Ladder, Output-Sensitive Bounds Warning

### Community 52 - "Streaming Sketches and FM-Index"
Cohesion: 0.25
Nodes (8): HyperLogLog, Mergeable Sketches (Ship the Sketch, Not the Estimate), Reservoir Sampling, The Streaming Model (One Pass, Sublinear Space), FM-Index / Burrows-Wheeler Transform, Ferragina & Manzini (2000) — FM-Index, fst (Finite-State Transducer, Succinct Key Set), Gallant — Index 1,600,000,000 Keys with Automata and Rust

### Community 53 - "Unicode Levels and Text Ranges"
Cohesion: 0.29
Nodes (8): String Matching, The Four Levels of "Character" (Bytes, Scalars, Graphemes, Words), Spolsky — The Absolute Minimum About Unicode, Strings & Text, UAX #29 — Unicode Text Segmentation, UTF-8 Validity Invariant / Char Boundaries, Suffix Structures, Half-Open [l, r) Range Convention

### Community 54 - "Leases, Audit, and Root Tokens"
Cohesion: 0.25
Nodes (8): Audit Devices, Leases, Renewal, and Revocation, OpenBao — Learning Notes, Pitfall: Lease and TTL Explosions, Pitfall: The Root Token That Never Died, OpenBao — Quick Reference, Bao Agent — File-Rendered Secrets, Day-1 Hardening (Audit First, Root Token Last)

### Community 55 - "Terraform State Drift Model"
Cohesion: 0.25
Nodes (8): OpenTofu — Learning Notes, Pitfall: Drift from Manual Changes, The Three-Way Model (Config, State, Reality), Divergence Classification (Name It, Then Fix It), OpenTofu — Quick Reference, Drift Detection (Scheduled, Not Discovered), Importing Existing Infrastructure, OpenTofu — Setup & Operations Runbook

### Community 56 - "Cache Invalidation and Idempotency Keys"
Cohesion: 0.33
Nodes (7): CDC-Driven Invalidation, Invalidation Strategies, TTL From a Stated Staleness Budget, Immutable / Versioned Keys, Deduplication Window and Consumer Contract, Idempotency Key, Natural Idempotency

### Community 57 - "Breaker Placement and Deferred Topics"
Cohesion: 0.29
Nodes (7): Breaker Placement: Per-Instance, Shared, or Mesh, Load Balancing & Service Discovery, Deferred Topics Policy, API Gateway & BFF (deferred), Deferred Topics Table, GPU Compute (deferred), Service Mesh (deferred)

### Community 58 - "Kruskal, DSU, and Edge Lists"
Cohesion: 0.29
Nodes (7): Edge List Representation, Measured CSR vs Vec<Vec<…>> (3.26×/1.76×), Matroid Structure, Kruskal's Algorithm (sort + DSU), Minimum Spanning Forest (disconnected input), DSU union() Return Value Is the Cycle Check, Measured Kruskal 7.4× Faster Than Lazy Prim

### Community 59 - "Sweep Line and Bucket Queues"
Cohesion: 0.29
Nodes (7): 0-1 BFS, Greedy's Cost Is the Sort, Bucket Queue (bounded integer priorities), Coordinate Compression, Max Overlap via +1/−1 Event Counter, Sweep Line, Sweep's Three Parts (events, status, rule)

### Community 60 - "Mergeable Quantiles and Top-k"
Cohesion: 0.33
Nodes (7): Mergeability (Merge Sketches, Then Query), t-digest / DDSketch (Mergeable Quantiles), Reservoir Sampling, Never Average Percentiles, Streaming Top-k via Size-k Min-Heap, The Fibonacci Heap Trap, Lazy Deletion in the Priority Queue

### Community 61 - "Type-Driven State Modelling"
Cohesion: 0.29
Nodes (7): Language Quick Reference Template, Practice: Model State with Enums, Not Booleans, Make Illegal States Unrepresentable, Practice: Parse, Don't Validate, Anti-Pattern: Primitive Obsession / Stringly-Typed State, Rust Do / Don't Table, Property Tests and Shrinking (proptest)

### Community 62 - "Consensus, Leases, Fencing"
Cohesion: 0.33
Nodes (6): Fencing Tokens, Leases, Linearizability from Consensus, Raft, Pitfall: The Zombie Projector, Formal Methods (TLA+) (deferred)

### Community 63 - "The Vec Invariant"
Cohesion: 0.33
Nodes (6): Capacity That Never Comes Back (clear() Doesn't Free), Contiguity, Not Indexing, Is the Array's Feature, The Rustonomicon: Implementing Vec, Take &[T], Not &Vec<T>, in Signatures, Stroustrup, Are Lists Evil?, The Vec Invariant (ptr, len, cap)

### Community 64 - "Hash Flooding and Randomized State"
Cohesion: 0.33
Nodes (6): Crosby & Wallach, Algorithmic Complexity Attacks (2003), Hash Flooding (HashDoS), Randomized Iteration Order, RandomState (per-instance seeded SipHash-1-3), Consistent Hashing (ring + virtual nodes), Hash Stability (never persist a randomized hash)

### Community 65 - "Heap Property and Heapify"
Cohesion: 0.33
Nodes (6): Comparator Consistency and Tiebreakers, Heap Property (partial order), Θ(n) Heapify (height counting), Sift Up / Sift Down, A Heap Is Not Sorted, Measured Heapify vs Push Loop (2.42×)

### Community 66 - "Rust Ownership Practices"
Cohesion: 0.33
Nodes (6): Language Learning Notes Template, Practice: Restructure Ownership Instead of Rc<RefCell<T>>, Anti-Pattern: .clone() as Borrow-Checker Duct Tape, Anti-Pattern: Holding a Lock Across .await, Practice: Move Data Between Threads Rather Than Sharing, Ownership as a Complete Function-Signature Contract

### Community 67 - "Error Design and Test Coverage"
Cohesion: 0.33
Nodes (6): Practice: Design Errors as Data, Not Strings, Practice: Accept General Borrowed Input, Return Owned Output, Anti-Pattern: Generic-izing Before Two Call Sites, Anti-Pattern: unwrap()/expect() on Fallible Paths, Pitfall: Chasing a Coverage Number (Goodhart), Pitfall: Not Testing the Error Paths

### Community 68 - "Container State and Volume Backups"
Cohesion: 0.33
Nodes (6): Migration Walkthrough: VM → Containers, Copy-on-Write Layer vs Volumes for State, Docker Migration Checklist, Disk Fills Silently — prune and Log Rotation, Docker Monitoring Signals Table, Volume Backups and Restore Drill

### Community 69 - "Connection Pooling and C10K"
Cohesion: 0.33
Nodes (6): Pitfall: Connection Exhaustion, Process-Per-Connection Model, Connection Pooling with PgBouncer (Not Optional), The C10K Problem (Thread-Per-Connection Rent), When Threads Win Anyway, Hoisting — Pay the Fixed Cost Once

### Community 70 - "Cache Lines and Working Sets"
Cohesion: 0.33
Nodes (6): Pitfall: Default Configuration in Production, Settings That Matter (Defaults Sized for a Laptop), Bump Arena (bumpalo) — Lifetime-Grouped Allocation, The Cache Line as the Unit of Transfer, Bandwidth Wall and Wasted Line Fraction, The Working-Set Cliff

### Community 71 - "Deadlines and Load Shedding"
Cohesion: 0.40
Nodes (5): Deadline Propagation, Load Shedding and Prioritization, Pitfall: Serving Dead Work, Pitfall: A Breaker With No Timeout, Fallbacks and Graceful Degradation

### Community 72 - "Cache-Aside and Snapshot Bootstrap"
Cohesion: 0.40
Nodes (5): Cache-Aside (Lazy Loading), Read-Through / Write-Through / Write-Behind, Hit-Rate Math, Snapshot + Streaming (Bootstrap Problem), Snapshot

### Community 73 - "Power of Two Choices"
Cohesion: 0.40
Nodes (5): Google SRE Book ch. 20 — Load Balancing in the Datacenter, Least-Request Balancing, Mitzenmacher, The Power of Two Choices in Randomized Load Balancing, Power of Two Random Choices (P2C), Load Balancing as a Decision Made with Stale Information

### Community 74 - "Quicksort and pdqsort Lineage"
Cohesion: 0.40
Nodes (5): Heapsort, Insertion Sort, Orson Peters — pdqsort: Pattern-defeating Quicksort, Quicksort, slice::sort_unstable (ipnsort / pdqsort)

### Community 75 - "Rebase vs Merge Discipline"
Cohesion: 0.40
Nodes (5): Cherry-Pick Is a Diff Applied Elsewhere, --force-with-lease, Pitfall: git pull Creating Merge Commits, Pitfall: Rebasing Shared History, Rebase Rewrites; Merge Records

### Community 76 - "Secret Rotation and Worktrees"
Cohesion: 0.40
Nodes (5): Committed Secret: Rotate First, git-filter-repo, Stashing, Git Worktrees, Git Gotchas Table

### Community 77 - "ripgrep Skipping and -u Ladder"
Cohesion: 0.40
Nodes (5): ripgrep GUIDE / burntsushi benchmark writeup, Skip Files Entirely (ripgrep's Central Design Decision), The -u Ladder, Debugging "Why Didn't It Match?" Ladder, -u Ladder Reference Table

### Community 78 - "Health Checks and Outlier Ejection"
Cohesion: 0.50
Nodes (4): Pitfall: Deep Readiness Probe Ejects the Whole Fleet, The Health-Check Split (Liveness vs Readiness vs Passive), Retry + Outlier Ejection Amplification Cascade, Panic Threshold (Ignore Health When >50% Unhealthy)

### Community 79 - "LCA and Heavy-Light Decomposition"
Cohesion: 0.50
Nodes (4): Bender & Farach-Colton, The LCA Problem Revisited (2000), LCA via Euler Tour + RMQ, Heavy-Light Decomposition, LCA by Binary Lifting

### Community 80 - "Bit Idioms and Submask Enumeration"
Cohesion: 0.50
Nodes (4): Lowest-Set-Bit Idioms (x & -x, x & (x-1)), Shift Overflow Pitfall, Submask Enumeration (Θ(3ⁿ)), Warren — Hacker's Delight

### Community 81 - "Asymptotics and Accidental Quadratics"
Cohesion: 0.50
Nodes (4): The Accidental Quadratic, Asymptotic Notation (O, Θ, Ω), Doubling Experiment, Pseudo-Polynomial Blowup (Θ(n·W))

### Community 82 - "Heavy Hitters and TinyLFU"
Cohesion: 0.50
Nodes (4): Count-Min Sketch, Misra-Gries Heavy Hitters, W-TinyLFU (Count-Min Admission Filter), Einziger et al. (2017) — TinyLFU

### Community 83 - "SIMD Substring Search"
Cohesion: 0.50
Nodes (4): memchr / memchr::memmem, Pitfall: Hand-Rolling KMP Instead of the Library, str::find (Two-Way + memchr SIMD Prefilter), Crochemore & Perrin (1991) — Two-Way String Matching

### Community 84 - "Percentiles and CI Noise Floors"
Cohesion: 0.50
Nodes (4): Exact Percentile by Sorting vs Mergeable Sketch, Log Processing Recipes (top IPs, p95 latency), iai-callgrind Instruction-Count CI Gate, Noise Floor and Small-Delta Distrust

### Community 85 - "OCI Lineage and Licensing"
Cohesion: 0.50
Nodes (4): OSS Tool Learning Notes Template, Docker's Real Innovation: Image Format, Buildfile, Registry, OCI / containerd / runc Lineage, Docker Desktop Licensing Trap

### Community 86 - "Provider Pinning and Upgrades"
Cohesion: 0.50
Nodes (4): The Dependency Graph, Pitfall: Unpinned Providers and Modules, Providers and the Resource Lifecycle, Provider and OpenTofu Version Upgrades (One-Way Door)

### Community 87 - "State File Layout and Surgery"
Cohesion: 0.50
Nodes (4): Pitfall: The Monolithic State File, State: The Crown Jewel, State Layout — Decide Before Twenty Services, State Surgery (Always Back Up First)

### Community 88 - "Schema Evolution Coupling"
Cohesion: 0.67
Nodes (3): Pitfall: Schema Coupling to DDL, Schema Contracts and Evolution, Pitfall: Event Schema Evolution (Immutability Trap)

### Community 89 - "Finishing the Migration"
Cohesion: 0.67
Nodes (3): The Deletion Plan (the Step That Realizes the Benefit), Never Finishing (the Dominant Failure Mode) and the Legacy Feature Freeze, The Shared Database That Never Gets Split

### Community 90 - "Memory Reclamation and ABA"
Cohesion: 0.67
Nodes (3): ABA Problem, Memory Reclamation (epochs, hazard pointers, RCU), crossbeam (epoch, channel, skiplist)

### Community 91 - "Hash/Eq Contract"
Cohesion: 0.67
Nodes (3): Hash/Eq Contract, a == b ⟹ hash(a) == hash(b), XOR-Combining Sub-Hashes Pitfall

### Community 92 - "Unicode Normalization and Collation"
Cohesion: 0.67
Nodes (3): Collation (Locale-Dependent Sort Order), UAX #15 — Unicode Normalization Forms, Unicode Normalization (NFC/NFD)

### Community 93 - "Suffix Array Construction (SA-IS)"
Cohesion: 0.67
Nodes (3): SA-IS (Linear Suffix Array Construction), The Sentinel Character Requirement, Nong, Zhang & Chan (2009) — SA-IS

### Community 94 - "Sliding Window Preconditions"
Cohesion: 0.67
Nodes (3): Monotonicity Precondition (Never Move a Pointer Back), Pitfall: Sliding Window with Negative Numbers, Prefix Sums + HashMap (The Negatives Fallback)

### Community 95 - "Git Three Areas and Worktrees"
Cohesion: 0.67
Nodes (3): git switch / git restore (2.23+), The Three Areas (Working Tree, Index, Repository), Git Worktrees

### Community 96 - "Searching Git History"
Cohesion: 0.67
Nodes (3): git bisect, Pickaxe Log Search (-S / -L / blame -C), Searching History (git log -S, git grep over rev-list)

### Community 97 - "sed Substitution Hazards"
Cohesion: 0.67
Nodes (3): Shell Expansion Eating the Pattern, Unescaped Delimiters and Injected Variables, The s/// Command and Its Flags

### Community 98 - "sed Addressing Model"
Cohesion: 0.67
Nodes (3): [address]command — Addresses Select, Commands Act, sed's Implicit Loop and Pattern Space, sed Addresses Table

### Community 99 - "Test Isolation and Determinism"
Cohesion: 0.67
Nodes (3): cargo-nextest (process-per-test isolation), Pitfall: Non-Deterministic Tests (seed, clock, HashMap order), Pitfall: Tests That Share Process State

### Community 100 - "Vault to OpenBao Migration"
Cohesion: 0.67
Nodes (3): Pitfall: Migrating from Vault on Vibes, Vault → OpenBao Migration Walkthrough, OpenBao Migration Checklist (from Vault)

### Community 101 - "TLB, Cache, Branch Signatures"
Cohesion: 0.67
Nodes (3): Branch Diagnostic Signatures, The TLB as a Second Cache (Huge Pages), Cache Diagnostic Signatures (perf stat / cachegrind)

## Ambiguous Edges - Review These
- `Immutable / Versioned Keys` → `Natural Idempotency`  [AMBIGUOUS]
  architecture-patterns/idempotency-and-delivery-semantics/learning.md · relation: semantically_similar_to
- `The Shard Key (the Dominating, Irreversible Decision)` → `SCC Condensation Yields a DAG`  [AMBIGUOUS]
  data-structures-and-algorithms/advanced-graph-algorithms/learning.md · relation: conceptually_related_to
- `Arena Allocation for Trees in Rust` → `Memoization (top-down)`  [AMBIGUOUS]
  data-structures-and-algorithms/dynamic-programming/reference.md · relation: conceptually_related_to
- `Machine Word as 64 Parallel Booleans` → `Exact / Adaptive Predicates`  [AMBIGUOUS]
  data-structures-and-algorithms/computational-geometry/reference.md · relation: conceptually_related_to
- `Orientation Test (orient(a,b,c))` → `Banded DP (distance ≤ k)`  [AMBIGUOUS]
  data-structures-and-algorithms/edit-distance-and-alignment/learning.md · relation: conceptually_related_to
- `Visited-Set Choice (Vec<bool> vs bitset vs HashSet)` → `Perfect Hashing (phf)`  [AMBIGUOUS]
  data-structures-and-algorithms/graph-traversal/learning.md · relation: conceptually_related_to
- `Visited-Set Choice (Vec<bool> vs bitset vs HashSet)` → `Write Stall (compaction falling behind)`  [AMBIGUOUS]
  data-structures-and-algorithms/lsm-trees/learning.md · relation: conceptually_related_to
- `Bloom Filter` → `Sqrt Decomposition`  [AMBIGUOUS]
  data-structures-and-algorithms/range-query-structures/learning.md · relation: conceptually_related_to
- `Average-Case vs Expected-Case` → `Dijkstra's Algorithm`  [AMBIGUOUS]
  data-structures-and-algorithms/shortest-paths/learning.md · relation: conceptually_related_to
- `The Streaming Model (One Pass, Sublinear Space)` → `FM-Index / Burrows-Wheeler Transform`  [AMBIGUOUS]
  data-structures-and-algorithms/suffix-structures/learning.md · relation: semantically_similar_to
- `UTF-8 Validity Invariant / Char Boundaries` → `Half-Open [l, r) Range Convention`  [AMBIGUOUS]
  data-structures-and-algorithms/two-pointers-and-sliding-window/learning.md · relation: semantically_similar_to
- `Practice: Move Data Between Threads Rather Than Sharing` → `SIGTERM Drain Handler and Stop Sequence`  [AMBIGUOUS]
  oss-tools/docker/runbook.md · relation: conceptually_related_to
- `PGO and BOLT` → `Granularity Knee`  [AMBIGUOUS]
  performance-optimization/compiler-optimizations/learning.md · relation: conceptually_related_to
- `Padding and Alignment` → `First-Touch Placement`  [AMBIGUOUS]
  performance-optimization/numa-awareness/learning.md · relation: conceptually_related_to
- `Granularity Knee` → `Tail Handling`  [AMBIGUOUS]
  performance-optimization/simd/learning.md · relation: conceptually_related_to

## Knowledge Gaps
- **322 isolated node(s):** `architecture-patterns Category`, `performance-optimization Category`, `data-structures-and-algorithms Category`, `Language Practice Docs (releasing/testing/benchmarking)`, `API Gateway & BFF (deferred)` (+317 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **31 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **What is the exact relationship between `Immutable / Versioned Keys` and `Natural Idempotency`?**
  _Edge tagged AMBIGUOUS (relation: semantically_similar_to) - confidence is low._
- **What is the exact relationship between `The Shard Key (the Dominating, Irreversible Decision)` and `SCC Condensation Yields a DAG`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._
- **What is the exact relationship between `Arena Allocation for Trees in Rust` and `Memoization (top-down)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._
- **What is the exact relationship between `Machine Word as 64 Parallel Booleans` and `Exact / Adaptive Predicates`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._
- **What is the exact relationship between `Orientation Test (orient(a,b,c))` and `Banded DP (distance ≤ k)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._
- **What is the exact relationship between `Visited-Set Choice (Vec<bool> vs bitset vs HashSet)` and `Perfect Hashing (phf)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._
- **What is the exact relationship between `Visited-Set Choice (Vec<bool> vs bitset vs HashSet)` and `Write Stall (compaction falling behind)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._