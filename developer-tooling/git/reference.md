# Git — Quick Reference

## Quick Facts

- **Does:** content-addressed object store (blob/tree/commit) + pointers (branches, HEAD)
- **Reach for it when:** anything versioned
- **Don't use it for:** large binaries (use LFS), secrets (rotate, don't just delete)
- **This machine:** git 2.50.1 (Apple Git-155)

## The Model

| Thing | Actually is |
| --- | --- |
| Branch | A file containing one commit hash |
| HEAD | Pointer to a branch (or a commit, when detached) |
| Commit | Immutable: tree + parent(s) + metadata |
| "Rewriting" | Making **new** commits and moving a pointer |
| Recoverable | Anything ever committed (via `reflog`) |
| **Not recoverable** | **Working-tree changes never committed or stashed** |

```
working tree ──add──▶ index ──commit──▶ repository
```

## Flags That Matter

| Flag | Does |
| --- | --- |
| `--force-with-lease` | Force-push **only if the remote hasn't moved** — always prefer over `--force` |
| `-x` (cherry-pick) | Record the source commit — do this for backports |
| `-m 1` (cherry-pick/revert) | Pick the parent to diff against, for merges |
| `--update-refs` (rebase) | Move dependent stacked branches too |
| `--autosquash` | Position `fixup!`/`squash!` commits automatically |
| `-S "str"` (log) | Commits that added/removed a string (pickaxe) |
| `-C` (blame) | Follow code moved between files |
| `...` (diff) | Changes **since divergence** — usually what you want |

## Syntax

```sh
# Undo, by intent
git restore --staged <f>       # unstage
git restore <f>                # discard WT changes — UNRECOVERABLE
git reset --soft HEAD~1        # undo commit, keep staged
git reset HEAD~1               # undo commit, keep unstaged
git reset --hard HEAD~1        # undo commit AND changes
git revert <sha>               # undo a PUSHED commit (new inverse commit)

# Recover
git reflog                     # every position HEAD has held
git reset --hard HEAD@{3}
git reset --hard ORIG_HEAD     # pre-rebase/merge tip

# Cherry-pick
git cherry-pick -x <sha>
git cherry-pick <start>^..<end>       # inclusive range
git cherry-pick -m 1 <merge-sha>

# Worktree — better than stashing for "check another branch"
git worktree add ../repo-hotfix hotfix
git worktree add -b fix ../repo-fix main
git worktree list && git worktree remove ../repo-hotfix

# Rewrite local history
git commit --amend --no-edit
git commit --fixup=<sha> && git rebase -i --autosquash <sha>^
git rebase -i HEAD~5

# Investigate
git log --oneline --graph --all --decorate
git log -S "needle"            git log -L 10,20:file.rs
git blame -w -C file.rs        git bisect start / bad / good <sha>
git diff main...feature        git range-diff main v1 v2
```

## Config Worth Setting

```sh
git config --global pull.ff only              # no surprise merge commits
git config --global merge.conflictstyle zdiff3 # shows the ORIGINAL text
git config --global rerere.enabled true        # replay past conflict resolutions
git config --global rebase.updateRefs true
git config --global diff.algorithm histogram
git config --global fetch.prune true
```

## Gotchas

| Gotcha | Fix |
| --- | --- |
| `reset --hard` ate uncommitted work | Unrecoverable — stash/commit before destructive ops |
| Rebased shared history | Duplicated commits for everyone; use `revert` on shared branches |
| `git pull` made a merge commit | `pull.ff only` or `pull.rebase true` |
| Detached HEAD commits vanished | `git reflog` → `git branch rescue <sha>` |
| Deleted secret still in history | Rotate first, then `git filter-repo` |
| `..` vs `...` in diff | Use `...` to review "what did this branch add" |
| Same branch in two worktrees | Not allowed — use `--detach` |
| `--force` clobbered a colleague | Always `--force-with-lease` |

## Key References

- [Pro Git](https://git-scm.com/book/en/v2) — ch. 7 & 10 for internals
- [git-filter-repo](https://github.com/newren/git-filter-repo)
- [Oh Shit, Git!?!](https://ohshitgit.com/)
