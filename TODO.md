# Wish — development tracker

Educational shell written in Rust. Detailed checklist for ongoing work.

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
- Exit status — last pipeline stage’s code kept in shell state; `$?` builtin prints it (teaching shortcut, not full `$` expansion)

## Docs

| Doc | Contents |
|-----|----------|
| [Interactive line editing](docs/interactive-line-editing.md) | Raw/cooked mode, key loop, Char/Backspace/Enter, cursor, redraw |
| [Command history](docs/command-history.md) | History store, Draft/History mode, Up/Down, push rules |
| [Execution](docs/execution.md) | Pipelines, redirection, EOF-after-redirect, exit status / `$?` |

## Later / non-goals (for now)

- Path tab-completion — complete files/dirs (starting simple)
- Persistent history file
- Ctrl-R reverse search
- Multi-line input
- Syntax highlighting
- Background jobs, quoting, env expansion (until explicitly planned)
