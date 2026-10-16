<h1 align="center">風 Kaze</h1>

<p align="center">
  <b>Frees disk space on macOS, starting with the Git worktrees you forgot about.</b><br>
  Native, fast, written in Rust with GPUI. Same dark frosted-glass look as <a href="https://github.com/ziracmo/mugetsu">Mugetsu</a>.
</p>

---

## Status

Early work. What exists today:

- the app window, theme and the list of project folders to watch.

Coming next:

1. **Dead worktree detection**: scan your project folders, list every worktree with its size
   (`node_modules`, `target` and build output included), and flag the ones whose folder is gone,
   whose branch is merged or deleted, or that nobody touched in a while.
2. **Cleanup**: sort by size, inspect, remove with `git worktree remove` and send leftovers to the
   Trash, always after a confirmation.
3. Other cleanups: Xcode caches, Docker, orphaned `node_modules`…

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
