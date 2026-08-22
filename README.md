# 🐚 Wish

A small Unix-like shell in **Rust** — built to learn how real shells work: processes, pipes, redirections, and an interactive line editor.

Not a bash clone. A clear, readable educational project you can run, read, and extend.

## Features

- **Pipelines** — `cmd1 | cmd2 | …` with proper stdin/stdout wiring
- **Redirection** — `<`, `>`, `>>` (file beats pipe; next stage gets EOF when stdout went to a file)
- **Interactive editing** — raw-mode prompt (crossterm), cursor Left/Right, Ctrl-C / Ctrl-D
- **History** — in-session Up/Down with draft save/restore
- **Builtins** — `cd`, `exit`, and `$?` for the last pipeline exit status
- **CWD prompt** — always know where you are

## Quick start

Needs a recent Rust toolchain (`rustc` + `cargo`):

```bash
# OPTIONAL: Skip this if you already have Rust toolchain installed
# macOS / Linux — see https://rustup.rs for other options
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Then from this repo:

```bash
cargo run
```

Try:

```bash
ls | wc -l
echo hello > out.txt
cat < out.txt
false
$?
```

## Caveats

Wish is intentionally simple. Known gaps vs a real shell:

- **Parsing** — stages split on ` | `; no quoting, escapes, or globs (`"a b"`, `*.rs` won’t work as in bash)
- **One redirect per stage** — e.g. `cmd < in > out` is not supported yet; use a single `<`, `>`, or `>>`
- **`$?`** — teaching builtin (type `$?` alone), not parameter expansion — `echo $?` does **not** expand
- **Exit status** — pipeline status is the **last** stage only (no `pipefail`); `cd`/spawn failures aren’t always reflected in `$?`
- **Builtins in pipelines** — e.g. `… | cd` doesn’t consume the pipe the way a child process would
- **No stderr redirects** — no `2>`, `2>&1`, here-docs, etc.
- **Line editor** — ASCII/byte-oriented cursor; history is in-session only
- **No jobs / advanced signals** — no `&`, job control, or full signal handling beyond editor Ctrl-C/D

## Design notes

Deeper write-ups live in [`docs/`](docs/):

- [Interactive line editing](docs/interactive-line-editing.md)
- [Command history](docs/command-history.md)
- [Execution](docs/execution.md) — pipelines, redirection, exit status / `$?`

Feature checklist: [`TODO.md`](TODO.md).

## Later goals

- Path tab-completion
- Persistent history / Ctrl-R
- Multi-line input
- Quoting & env expansion
- Background jobs
- Syntax highlighting

## License

Use and learn freely — this is an educational prototype.
