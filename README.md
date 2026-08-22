# Wish

Educational shell written in Rust.

## Done

- Prompt with current working directory
- External commands and pipelines (`cmd1 | cmd2 | ...`)
- Builtins: `cd`, `exit`
- Raw-mode line editor (crossterm): printable chars, Backspace, Enter, Esc
- Left / Right cursor; insert and backspace in the middle of the line
- Ctrl-C (cancel line → new prompt) and Ctrl-D on empty line (exit)
- Full-line redraw (`prompt` + `input_line`) with cursor placement
- In-session command history (cap, skip duplicate last)
- Up / Down history browse with draft save/restore
- `TerminalRawMode` RAII guard (raw while typing; cooked on drop / before execute)
- Redirection — `<` (stdin), `>` (truncate stdout), `>>` (append stdout); open-fail aborts
- After a stage redirects stdout to a file, the next pipeline stage gets EOF stdin (`Stdio::null`), not the terminal

## Next Up

Priority order for upcoming work:

1. **Exit status** — use last pipeline stage status; optional `$?`
2. **Path tab-completion** — complete files/dirs (starting simple)

## Docs

| Doc | Contents |
|-----|----------|
| [Interactive line editing](docs/interactive-line-editing.md) | Raw/cooked mode, key loop, Char/Backspace/Enter, redraw |
| [Command history](docs/command-history.md) | History store, Draft/History mode, Up/Down, push rules |

## Later / non-goals (for now)

- Persistent history file
- Ctrl-R reverse search
- Multi-line input
- Syntax highlighting
- Background jobs, quoting, env expansion (until explicitly planned)
