# Notes for AI sessions

- Look and feel follow Mugetsu (`ziracmo/mugetsu`): same `assets/theme.json`, glass palette in
  `crates/kaze-app/src/theme.rs`, UI strings as `tr("français", "English")`.
- Engine code goes in `kaze-core` (no GPUI), with tests against real repos built with the git CLI.
- Processes go through `kaze_core::process::command` (clippy forbids a bare `Command::new`).
- Deleting anything is reversible (Trash) and always confirmed by the user.
- Commits are authored by Alexandre (`12600085+ziracmo@users.noreply.github.com`), with no AI trailers.
