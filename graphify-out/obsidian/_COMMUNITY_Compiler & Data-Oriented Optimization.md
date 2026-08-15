---
type: community
cohesion: 0.07
members: 73
---

# Compiler & Data-Oriented Optimization

**Cohesion:** 0.07 - loosely connected
**Members:** 73 nodes

## Members
- [[serde(borrow)]] - rationale - performance-optimization/serialization-and-encoding/learning.md
- [[&mut as noalias]] - rationale - performance-optimization/compiler-optimizations/learning.md
- [[-Zprint-type-sizes]] - rationale - performance-optimization/memory-layout/learning.md
- [[Access-in-Place (rkyv)]] - rationale - performance-optimization/serialization-and-encoding/learning.md
- [[AoS to SoA Transformation]] - rationale - performance-optimization/data-oriented-design/learning.md
- [[Autovectorization]] - rationale - performance-optimization/simd/learning.md
- [[Borrowed Views]] - rationale - performance-optimization/zero-copy/learning.md
- [[Bounds-Check Elision]] - rationale - performance-optimization/compiler-optimizations/learning.md
- [[Choose Per Boundary, Not Per Project]] - rationale - performance-optimization/serialization-and-encoding/learning.md
- [[Compiler Optimizations — Learning]] - document - performance-optimization/compiler-optimizations/learning.md
- [[Compiler Optimizations — Quick Reference]] - document - performance-optimization/compiler-optimizations/reference.md
- [[Compression as a Fourth Axis]] - rationale - performance-optimization/serialization-and-encoding/learning.md
- [[Copy Cost Model]] - rationale - performance-optimization/zero-copy/learning.md
- [[Data-Oriented Design — Learning]] - document - performance-optimization/data-oriented-design/learning.md
- [[Data-Oriented Design — Quick Reference]] - document - performance-optimization/data-oriented-design/reference.md
- [[Decode Cost Model]] - rationale - performance-optimization/serialization-and-encoding/learning.md
- [[ECS as Productized DoD]] - rationale - performance-optimization/data-oriented-design/learning.md
- [[Encode into Reused Buffers]] - rationale - performance-optimization/serialization-and-encoding/learning.md
- [[Enum-in-Vec over VecBoxdyn T]] - rationale - performance-optimization/data-oriented-design/learning.md
- [[Existence-Based Processing]] - rationale - performance-optimization/data-oriented-design/learning.md
- [[Fat Enum Variant Boxing]] - rationale - performance-optimization/memory-layout/learning.md
- [[Float Reduction Semantics]] - rationale - performance-optimization/simd/learning.md
- [[Gather-Write]] - rationale - performance-optimization/zero-copy/learning.md
- [[Generational Handle Identity]] - rationale - performance-optimization/data-oriented-design/learning.md
- [[Handle Size Menu]] - rationale - performance-optimization/memory-layout/learning.md
- [[Hardware Counters and IPC]] - rationale - performance-optimization/profiling-and-measurement/learning.md
- [[HotCold Field Split]] - rationale - performance-optimization/data-oriented-design/learning.md
- [[Inlining as the Gateway Optimization]] - rationale - performance-optimization/compiler-optimizations/learning.md
- [[Instruction-Count Regression Gates]] - rationale - performance-optimization/profiling-and-measurement/learning.md
- [[Intrinsics and Runtime Dispatch]] - rationale - performance-optimization/simd/learning.md
- [[Kernel-Side Moves]] - rationale - performance-optimization/zero-copy/learning.md
- [[LTO and codegen-units=1]] - rationale - performance-optimization/compiler-optimizations/learning.md
- [[Lane Parallelism]] - rationale - performance-optimization/simd/learning.md
- [[Layout Luck]] - rationale - performance-optimization/profiling-and-measurement/learning.md
- [[Line Efficiency]] - rationale - performance-optimization/memory-layout/learning.md
- [[Masks Replace Branches]] - rationale - performance-optimization/simd/learning.md
- [[Memory Layout — Learning]] - document - performance-optimization/memory-layout/learning.md
- [[Memory Layout — Quick Reference]] - document - performance-optimization/memory-layout/reference.md
- [[Monomorphization vs dyn Opacity]] - rationale - performance-optimization/compiler-optimizations/learning.md
- [[Niche Optimization]] - rationale - performance-optimization/memory-layout/learning.md
- [[Off-CPU Analysis]] - rationale - performance-optimization/profiling-and-measurement/learning.md
- [[PGO and BOLT]] - rationale - performance-optimization/compiler-optimizations/learning.md
- [[Padding and Alignment]] - rationale - performance-optimization/memory-layout/learning.md
- [[Portable SIMD]] - rationale - performance-optimization/simd/learning.md
- [[Profiling and Measurement — Learning]] - document - performance-optimization/profiling-and-measurement/learning.md
- [[Profiling and Measurement — Quick Reference]] - document - performance-optimization/profiling-and-measurement/reference.md
- [[Protobuf Field-Number Evolution]] - rationale - performance-optimization/serialization-and-encoding/learning.md
- [[Refcounted Views (bytesBytes)]] - rationale - performance-optimization/zero-copy/learning.md
- [[Report Distributions, Not Points]] - rationale - performance-optimization/profiling-and-measurement/learning.md
- [[SIMD Roofline]] - rationale - performance-optimization/simd/learning.md
- [[SIMD — Learning]] - document - performance-optimization/simd/learning.md
- [[SIMD — Quick Reference]] - document - performance-optimization/simd/reference.md
- [[Sampling Profilers and Flamegraphs]] - rationale - performance-optimization/profiling-and-measurement/learning.md
- [[Serialization and Encoding — Learning]] - document - performance-optimization/serialization-and-encoding/learning.md
- [[Serialization and Encoding — Quick Reference]] - document - performance-optimization/serialization-and-encoding/reference.md
- [[Stay Vertical]] - rationale - performance-optimization/simd/learning.md
- [[Tail Handling]] - rationale - performance-optimization/simd/learning.md
- [[The Amplification Trap]] - rationale - performance-optimization/zero-copy/learning.md
- [[The Durable-Format Immutability Trap]] - rationale - performance-optimization/serialization-and-encoding/learning.md
- [[The Knobs Ladder]] - rationale - performance-optimization/compiler-optimizations/learning.md
- [[The Measurement Funnel]] - rationale - performance-optimization/profiling-and-measurement/learning.md
- [[The Microbenchmark Mirage]] - rationale - performance-optimization/profiling-and-measurement/learning.md
- [[Three-Axis Format Trade-off]] - rationale - performance-optimization/serialization-and-encoding/learning.md
- [[Transform-Shaped APIs]] - rationale - performance-optimization/data-oriented-design/learning.md
- [[Views for Transit, Copies for Retention]] - rationale - performance-optimization/zero-copy/learning.md
- [[Zero-Copy — Learning]] - document - performance-optimization/zero-copy/learning.md
- [[Zero-Copy — Quick Reference]] - document - performance-optimization/zero-copy/reference.md
- [[Zero-Cost Abstraction as a Contract]] - rationale - performance-optimization/compiler-optimizations/learning.md
- [[black_box and Dead Code Elimination]] - rationale - performance-optimization/profiling-and-measurement/learning.md
- [[mmap Trade-offs]] - rationale - performance-optimization/zero-copy/learning.md
- [[repr(C) vs repr(Rust)]] - rationale - performance-optimization/memory-layout/learning.md
- [[repr(packed) Trap]] - rationale - performance-optimization/memory-layout/learning.md
- [[target-cpu=native]] - rationale - performance-optimization/compiler-optimizations/learning.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Compiler__Data-Oriented_Optimization
SORT file.name ASC
```

## Connections to other communities
- 19 edges to [[_COMMUNITY_False Sharing and Coherence]]

## Top bridge nodes
- [[Profiling and Measurement — Learning]] - degree 18, connects to 1 community
- [[Memory Layout — Learning]] - degree 17, connects to 1 community
- [[Data-Oriented Design — Learning]] - degree 16, connects to 1 community
- [[SIMD — Learning]] - degree 16, connects to 1 community
- [[Zero-Copy — Learning]] - degree 13, connects to 1 community