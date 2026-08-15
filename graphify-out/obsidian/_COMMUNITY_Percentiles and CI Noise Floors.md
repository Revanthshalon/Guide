---
type: community
cohesion: 0.50
members: 4
---

# Percentiles and CI Noise Floors

**Cohesion:** 0.50 - moderately connected
**Members:** 4 nodes

## Members
- [[Exact Percentile by Sorting vs Mergeable Sketch]] - rationale - developer-tooling/sed-and-text-processing/recipes.md
- [[Log Processing Recipes (top IPs, p95 latency)]] - concept - developer-tooling/sed-and-text-processing/recipes.md
- [[Noise Floor and Small-Delta Distrust]] - concept - language-best-practices/rust/benchmarking.md
- [[iai-callgrind Instruction-Count CI Gate]] - concept - language-best-practices/rust/benchmarking.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Percentiles_and_CI_Noise_Floors
SORT file.name ASC
```
