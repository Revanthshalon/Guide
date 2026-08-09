# Git — Recipes

> Concepts are in [learning.md](learning.md); flag lookup is in [reference.md](reference.md).
> **The one rule that makes everything below safe:** if it was ever committed, it's recoverable via `git reflog`. If it was only ever in the working tree, it is not — so commit or stash before anything destructive.

## Cherry-picking

### I want one commit from another branch

```sh
git cherry-pick <sha>
git cherry-pick -x <sha>          # records "(cherry picked from commit …)" — do this for backports
```

### A range of commits

```sh
git cherry-pick <start>..<end>    # EXCLUSIVE of <start>
git cherry-pick <start>^..<end>   # inclusive of <start> — the usual intent
```

### Cherry-pick without committing (to stage and edit first)

```sh
git cherry-pick -n <sha>          # applies to index + working tree, no commit
```

### It conflicted

```sh
# fix the files, then:
git add <files>
git cherry-pick --continue
# or
git cherry-pick --abort           # back to before you started
git cherry-pick --skip            # drop this one, continue the range
```

### Cherry-pick a merge commit

```sh
git cherry-pick -m 1 <merge-sha>  # -m 1 = "diff against the FIRST parent" (usually main)
```

A merge has two parents, so Git can't infer which diff you mean — hence the mandatory `-m`.

## Worktrees

### I need another branch checked out right now, without disturbing this one

```sh
git worktree add ../myrepo-hotfix hotfix/urgent   # existing branch
git worktree add -b fix-123 ../myrepo-fix main    # new branch from main
```

Both directories share one object store — no re-clone, no duplicate history, and each has its own HEAD and index.

### Manage them

```sh
git worktree list
git worktree remove ../myrepo-hotfix     # refuses if there are uncommitted changes
git worktree prune                       # clean up records of manually-deleted dirs
```

### Review a PR without touching your work

```sh
git fetch origin pull/42/head:pr-42
git worktree add ../review-42 pr-42
# review, build, run tests there; delete when done
```

**Constraint:** the same branch cannot be checked out in two worktrees. Use a detached checkout if you need the same commit twice: `git worktree add --detach ../tmp <sha>`.

## Rewriting local history

### Fix the last commit

```sh
git commit --amend                        # edit message and/or add staged changes
git commit --amend --no-edit              # keep the message, just fold in staged changes
```

### Reorder, squash, drop, or edit older commits

```sh
git rebase -i HEAD~5
# pick / reword / edit / squash (keep both messages) / fixup (discard this message) / drop
```

### Amend a commit that isn't the last one

```sh
git commit --fixup=<sha>                  # creates "fixup! …"
git rebase -i --autosquash <sha>^         # positions it automatically
```

### Split one commit into two

```sh
git rebase -i <sha>^        # mark it `edit`
git reset HEAD^             # unstage its changes, keep them in the working tree
git add -p                  # stage the first half interactively
git commit -m "first half"
git commit -am "second half"
git rebase --continue
```

### Keep stacked branches consistent while rebasing

```sh
git rebase --update-refs main   # moves dependent branch pointers too (git 2.38+)
```

## Undo — by what you want undone

| I want to… | Command | Destroys work? |
| --- | --- | --- |
| Unstage a file | `git restore --staged <f>` | no |
| Discard working-tree changes to a file | `git restore <f>` | **yes, unrecoverable** |
| Undo the last commit, keep changes staged | `git reset --soft HEAD~1` | no |
| Undo the last commit, keep changes unstaged | `git reset HEAD~1` | no |
| Undo the last commit and the changes | `git reset --hard HEAD~1` | **yes** (commit recoverable, WT changes not) |
| Undo a **pushed** commit | `git revert <sha>` | no — makes a new inverse commit |
| Undo a pushed merge | `git revert -m 1 <merge-sha>` | no |
| Throw away everything uncommitted | `git reset --hard && git clean -fd` | **yes, unrecoverable** |

`revert` is the right tool for shared history: it adds a commit rather than rewriting, so nobody else has to reconcile.

## Recovery — "I did something wrong"

### I lost commits (bad reset / rebase / deleted branch)

```sh
git reflog                                 # every position HEAD has held
git reset --hard HEAD@{3}                  # go back to entry 3
git branch rescue <sha>                    # or re-attach the commits to a new branch
```

### I deleted a branch

```sh
git reflog | grep -i "branch-name"         # find its last tip
git branch branch-name <sha>
```

### A rebase went wrong

```sh
git rebase --abort                         # if still in progress
git reset --hard ORIG_HEAD                 # if it finished — ORIG_HEAD is the pre-rebase tip
```

### I force-pushed over someone's work

```sh
git reflog show origin/main                # if you have it fetched
# otherwise ask them for their local sha and:
git push --force-with-lease origin <their-sha>:main
```

Use `--force-with-lease` always — it refuses when the remote moved since your last fetch, which is exactly the check that prevents this.

### I committed a secret

1. **Rotate the credential first.** History rewriting is slow; assume it's already compromised.
2. Then rewrite:
```sh
pip install git-filter-repo
git filter-repo --path secrets.env --invert-paths     # or --replace-text
git push --force --all && git push --force --tags
```
3. Everyone re-clones. Add `gitleaks` to pre-commit and CI.

### I committed to the wrong branch

```sh
git branch correct-branch          # mark the current commits
git reset --hard origin/main       # rewind the wrong branch
git switch correct-branch
```

### What did I do? / who changed this?

```sh
git log --oneline --graph --all --decorate
git log -p <file>                  # history of one file, with diffs
git log -S "someString"            # commits that ADDED or REMOVED that string ("pickaxe")
git log -L 10,20:src/main.rs       # history of specific lines
git blame -w -C <file>             # -w ignores whitespace, -C follows moved code
git bisect start; git bisect bad; git bisect good <sha>   # binary search for the breaking commit
```

`git bisect` is [binary search](../../data-structures-and-algorithms/binary-search/learning.md) over commits — the predicate "is the bug present" is monotone, which is exactly its precondition.

## Stashing

```sh
git stash push -m "wip: parser"       # -m so `stash list` is readable
git stash push --include-untracked    # -u: untracked files too (otherwise they're left behind)
git stash push -- src/parser.rs       # only specific paths
git stash list
git stash show -p stash@{1}
git stash apply stash@{1}             # keep the stash entry
git stash pop                         # apply and drop
git stash branch fix-parser stash@{0} # apply onto a new branch from where it was created
```

Prefer a **worktree** over stashing for "I need to look at another branch" — no state to restore later.

## Inspecting and comparing

```sh
git diff                      # working tree vs index
git diff --staged             # index vs HEAD  (what `git commit` will record)
git diff main...feature       # changes on feature SINCE it diverged (three dots — usually what you want)
git diff main..feature        # difference between the two tips
git show <sha>                # one commit
git range-diff main feature-v1 feature-v2   # how a rebased/amended branch changed
```

The `...` vs `..` distinction is the one people get wrong: for reviewing a branch, **three dots** answers "what did this branch add".

## Configuration worth setting

```sh
git config --global pull.ff only                  # never a surprise merge commit
git config --global rebase.autosquash true
git config --global rebase.updateRefs true        # keep stacked branches consistent
git config --global diff.algorithm histogram      # better diffs than the default
git config --global merge.conflictstyle zdiff3    # shows the ORIGINAL text in conflicts
git config --global rerere.enabled true           # remember conflict resolutions and replay them
git config --global fetch.prune true              # drop local refs for deleted remote branches
git config --global init.defaultBranch main
git config --global push.default simple
```

**`zdiff3` and `rerere` are the two most under-used.** `zdiff3` shows what the text looked like *before* both sides changed it, which usually makes the correct resolution obvious. `rerere` records how you resolved a conflict and reapplies it automatically the next time the same conflict appears — invaluable during a long rebase.

## References

- [Pro Git](https://git-scm.com/book/en/v2) — free; chapters 7 and 10 (internals) are what make the model click
- [git-filter-repo](https://github.com/newren/git-filter-repo) — the maintained history-rewriting tool
- [Oh Shit, Git!?!](https://ohshitgit.com/) — task-oriented recovery, same spirit as this file
