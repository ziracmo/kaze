<h1 align="center">風 Kaze</h1>

<p align="center">
  <b>Frees disk space on macOS, starting with the Git worktrees you forgot about.</b><br>
  Native, fast, written in Rust with GPUI. Same dark frosted-glass look as <a href="https://github.com/ziracmo/mugetsu">Mugetsu</a>.
</p>

---

## Status

Early work. What exists today:

- **Dead worktree detection.** Kaze scans your project folders for Git repositories, lists every
  linked worktree with its real size on disk (`node_modules`, `target` and build output included),
  and flags it as dead when:
  - its folder is gone but git still lists it,
  - its branch is merged into `origin/HEAD` or the local `main`/`master`,
  - its branch was deleted on the remote (a merged pull request, squash merges included),
  - nobody touched it for 30 days.

  Uncommitted changes, unpushed commits and locked worktrees are shown so nothing is lost.

Coming next:

1. **Cleanup**: sort by size, inspect, remove with `git worktree remove` and send leftovers to the
   Trash, always after a confirmation.
2. Other cleanups: Xcode caches, Docker, orphaned `node_modules`…

## Build

Needs Rust (stable) on macOS 12+.

```sh
cargo run -p kaze-app            # debug run
scripts/bundle-macos.sh          # build Kaze.app and install it in /Applications
scripts/dmg-macos.sh             # pack target/release/Kaze.app into Kaze.dmg
cargo test --workspace           # tests (also run headless on Linux in CI)
```

## Layout

| Crate | What it does |
| --- | --- |
| `kaze-core` | The engine: scanning, sizes, cleanup. No UI, fully testable. |
| `kaze-app` | The GPUI app. |

## License

MIT
