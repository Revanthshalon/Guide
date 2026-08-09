# Git — Learning Notes

## Mental Model

**Git is a content-addressed object store with a few pointers on top.** Nearly every confusing thing about Git becomes predictable once you hold four object types and two pointers:

| Object | Is | Named by |
| --- | --- | --- |
| **blob** | file contents (no name, no path) | SHA-1/SHA-256 of the content |
| **tree** | a directory: names → blobs/trees | hash of its listing |
| **commit** | a tree + parent(s) + author + message | hash of all of that |
| tag | a named pointer to an object | hash |

And:

- A **branch** is a 41-byte file containing a commit hash. That's all. Creating one is free; deleting one deletes a pointer, not commits.
- **HEAD** is a pointer to a branch (or, when "detached", directly to a commit).

From this, the behaviour follows:

- **Commits are immutable.** `git commit --amend`, `rebase`, and `reset` do not modify commits — they create *new* ones and move a pointer. The old commits still exist until garbage collection, which is why almost everything is recoverable ([recipes](recipes.md)).
- **A branch "containing" commits** just means the commit is reachable by walking parent links from that branch's tip.
- **Merging vs rebasing** is a choice about which *shape* of history to create, not about which changes end up in the tree.

The second model that removes most day-to-day confusion is the **three areas**:

```
working tree  ──git add──▶  index (staging area)  ──git commit──▶  repository
     ▲                            ▲                                    │
     └────git checkout/restore────┴──────────git reset ────────────────┘
```

The **index** is a real file (`.git/index`), not a concept — a snapshot of what the next commit will contain. Most "why didn't my change get committed" questions are the index doing exactly what it was told.

## The Model in Detail

### Reachability and garbage collection

- **What it is:** An object survives if it's reachable from a *ref* (branch, tag, HEAD) or from the **reflog**. Unreachable objects are pruned by `git gc`, typically after 30 days for loose objects (90 for reflog entries).
- **Why it matters in practice:** This is why a "lost" commit after a bad `reset --hard` is almost always still there — `git reflog` lists where HEAD has pointed, and `git reset --hard <sha>` brings it back. **The rule of thumb: if it was ever committed, it's recoverable; if it was only ever in the working tree, it is not.**

### Rebase rewrites; merge records

- **What it is:** `merge` creates a commit with two parents, preserving both histories. `rebase` replays your commits onto a new base, creating *new commits with new hashes*.
- **Why it matters in practice:** Rebasing commits that others have pulled forces them to reconcile two versions of "the same" work — the reason for the rule *never rebase shared history*. Locally, rebasing before pushing produces a linear history that `git bisect` and `git log` are much easier to read.

### Cherry-pick is a diff, applied elsewhere

- **What it is:** `git cherry-pick <sha>` computes the diff that commit introduced and applies it to your current HEAD as a **new commit with a new hash**.
- **Why it matters in practice:** The same change now exists twice in history under two hashes. If the branches later merge, Git usually resolves it (the content matches), but it can produce spurious conflicts — which is why `-x` (recording "cherry picked from …") matters for traceability, and why cherry-picking should be a deliberate exception rather than a workflow.

### Worktrees — one repository, several checkouts

- **What it is:** `git worktree add ../hotfix main` creates a *second working directory* backed by the same `.git` object store, with its own HEAD and index.
- **Why it matters in practice:** It replaces the stash-switch-stash dance for the common "I need to look at another branch right now" case, and it's genuinely better for parallel work: a long build in one worktree isn't disturbed by checking out another branch elsewhere. The constraint is that **the same branch cannot be checked out in two worktrees**, which prevents two directories fighting over one ref.

### The reflog is the safety net

- **What it is:** A per-ref log of every position HEAD (and each branch) has held, with timestamps. Local only — never pushed.
- **Why it matters in practice:** It is the undo history for operations that "lose" commits: bad rebase, bad reset, deleted branch, botched amend. Knowing `git reflog` exists converts most Git disasters into a two-command fix.

### Detached HEAD is a state, not an error

- **What it is:** HEAD points directly at a commit rather than at a branch. Commits made here are reachable only from HEAD.
- **Why it matters in practice:** Checking out a tag or an old commit detaches HEAD, and commits made there are lost when you switch away *unless* you create a branch. The message says exactly this and is widely ignored — the fix is `git switch -c <name>` before or after.

## Portability & Variants

**This machine: git 2.50.1 (Apple Git-155).** Version matters more than usual for Git because the modern commands are recent additions:

| Modern | Legacy | Since |
| --- | --- | --- |
| `git switch` | `git checkout <branch>` | 2.23 |
| `git restore` | `git checkout -- <file>` | 2.23 |
| `git rebase --update-refs` | manual stacked-branch fixups | 2.38 |
| `--force-with-lease` | `--force` | long-standing, still under-used |

`switch`/`restore` exist because `checkout` did two unrelated jobs (move HEAD, overwrite files) and the overlap caused data loss. Prefer them.

**Apple Git** lags upstream by several releases and omits some optional features; `brew install git` gets you current if you need recent flags.

## Pitfalls in Depth

### Pitfall: `git reset --hard` losing uncommitted work

- **What goes wrong:** `git reset --hard` discards uncommitted changes in the working tree and index. Unlike almost every other Git operation, **this is not recoverable** — those bytes were never in an object.
- **Why it happens (the mechanism):** Git's recoverability comes from content being written into the object store. Uncommitted changes exist only as files on disk; `--hard` overwrites them from the target commit. The reflog records where *refs* pointed, not what your working tree contained.
- **How to handle it, and why that works:** Before any destructive operation, `git stash --include-untracked` or `git commit --no-verify -m wip` — either one writes the content into the object store, at which point the reflog can recover it. For the specific case of "I want to discard *some* changes", use `git restore <path>` so the blast radius is a path rather than the whole tree.
- **Trade-offs of the fix:** A stash-first habit adds a step and leaves stash entries to clean up. The alternative — losing an hour of work once — is a worse trade.

### Pitfall: Rebasing shared history

- **What goes wrong:** A branch that others have pulled is rebased and force-pushed. Everyone else's copy now has the *old* commits, and their next `git pull` merges the two versions together, duplicating every commit and producing conflicts that look inexplicable.
- **Why it happens (the mechanism):** Rebase creates new commits with new hashes for the same changes. Git has no way to know that `abc123` and `def456` are "the same work", so it treats them as divergent history and merges both — the duplication is Git behaving correctly on a history that was rewritten underneath it.
- **How to handle it, and why that works:** Rebase only commits that exist solely in your local repository. For shared branches use merge. When a force-push is genuinely required (a squashed PR branch), use **`--force-with-lease`**, which refuses if the remote moved since you last fetched — that check is what prevents overwriting a colleague's push, and it's the entire reason to prefer it over `--force`.
- **Trade-offs of the fix:** Merge-based history is noisier and harder to bisect. Teams that want linear history usually rebase *before* the first push and squash-merge at the PR boundary, which keeps rewriting private.

### Pitfall: Committing secrets

- **What goes wrong:** An API key is committed and pushed. Deleting it in a later commit does not remove it — the blob is still in history and in every clone, and it's retrievable with `git log -p` or by hash.
- **Why it happens (the mechanism):** Git is append-only and content-addressed; a commit that "removes" a file records a tree without it, but the blob remains reachable from the earlier commit. This is the same immutability as Docker image layers ([Docker](../../oss-tools/docker/learning.md)) — deletion in a later layer doesn't remove the content.
- **How to handle it, and why that works:** **Rotate the credential first** — history rewriting is slow and the secret is already exposed, so treating it as compromised is the only safe assumption. Then rewrite with `git filter-repo` (the maintained replacement for `filter-branch`) or BFG, force-push all refs, and have everyone re-clone. Prevent recurrence with `gitleaks`/`trufflehog` in a pre-commit hook and in CI.
- **Trade-offs of the fix:** Rewriting history invalidates every existing clone and every open PR, and on a busy repository that's genuinely disruptive. If the secret is already rotated, some teams reasonably choose to leave history alone and rely on rotation — the calculus is whether the historical value of the secret is zero.

### Pitfall: `git pull` creating merge commits unintentionally

- **What goes wrong:** `git pull` on a branch that has diverged creates a merge commit, so a feature branch accumulates "Merge branch 'main' into feature" noise that makes review and bisect harder.
- **Why it happens (the mechanism):** `git pull` is `fetch` + `merge` by default. When your branch and the remote have both advanced, a merge is the only way to reconcile them without rewriting — so Git does exactly what it was configured to do.
- **How to handle it, and why that works:** Set `git config --global pull.rebase true` (rebase local commits onto the fetched tip) or `pull.ff only` (fail if a merge would be needed, forcing an explicit decision). `ff only` is the most predictable default because it never silently changes history shape.
- **Trade-offs of the fix:** `pull.rebase true` rewrites your local commits on every pull, which is fine for private work and wrong if you've already pushed the branch and others use it. `ff only` requires you to handle divergence manually every time, which is more friction but no surprises.

### Pitfall: Detached HEAD losing commits

- **What goes wrong:** `git checkout <tag>` or `git checkout <sha>` detaches HEAD. Work is committed there, then a branch is checked out, and the commits vanish from every view — no branch points at them.
- **Why it happens (the mechanism):** Commits are reachable from refs. In detached HEAD there is no branch to advance, so HEAD is the only thing pointing at the new commits; moving HEAD elsewhere leaves them unreachable and eventually collectable.
- **How to handle it, and why that works:** Create a branch as soon as you intend to commit: `git switch -c experiment`. If you've already switched away, `git reflog` shows the abandoned commits and `git branch rescue <sha>` re-attaches them — this works because the reflog is itself a ref that keeps them reachable.
- **Trade-offs of the fix:** None meaningful. The reflog window (default 90 days for reachable, 30 for unreachable) is the only limit, and it's long enough in practice.
