# Interactive line editing

How Wish collects a command line before execution.

The prompt runs in **raw mode** so each keystroke is handled immediately. A `cursor_pos` marks the caret (ASCII / byte-oriented). Insert and Backspace act at the caret, not only at the end of the line.

Related: [Command history](command-history.md) · [Execution](execution.md)

## Flow

```mermaid
flowchart TD
  START([Shell start]) --> PROMPT

  PROMPT[Show prompt] --> RAW[Enable raw mode]
  RAW --> RESET[Clear line / draft state]
  RESET --> READ[Read key event]

  READ --> MATCH{Key?}

  MATCH -->|Printable char| CHAR[Insert at cursor + redraw]
  MATCH -->|Backspace| BS[Delete before cursor + redraw]
  MATCH -->|Left / Right| CUR[Move cursor + redraw]
  MATCH -->|Up / Down| HIST[History browse]
  MATCH -->|Ctrl-C| CANCEL[Cancel → new prompt]
  MATCH -->|Ctrl-D empty| EXITD([Exit shell])
  MATCH -->|Enter| ENTER{Line empty?}
  MATCH -->|Esc| EXIT([Exit shell])
  MATCH -->|Other| NOOP[No-op]
  NOOP --> READ

  CHAR --> READ
  BS --> READ
  CUR --> READ
  HIST --> READ
  CANCEL --> PROMPT

  ENTER -->|yes| PROMPT
  ENTER -->|no| COOKED[Leave raw / cooked mode]
  COOKED --> EXEC[History push + execute]
  EXEC --> EXITCHK{exit?}
  EXITCHK -->|yes| DONE([Exit])
  EXITCHK -->|no| PROMPT
```

## Raw vs cooked

| Phase | Mode | Why |
|-------|------|-----|
| Editing the line | **Raw** | Keystrokes arrive one at a time; the shell draws the line itself |
| Running a command | **Cooked** | Child processes expect normal terminal line discipline |

Wish uses a RAII guard (`TerminalRawMode`): raw while the guard lives; cooked again when it drops — including before execute.

## Keys

| Input | Behavior |
|-------|----------|
| Printable character | Insert at caret, move caret right, redraw |
| Backspace | Delete the character before the caret (no-op at column 0) |
| Left / Right | Move caret within the line |
| Up / Down | Browse history ([details](command-history.md)); caret jumps to end of the loaded line |
| Ctrl-C | Discard the line → new prompt |
| Ctrl-D (empty line) | Exit the shell |
| Enter | Submit (below) |
| Esc | Exit the shell |

## Submit

1. Move to the next screen line (`\r\n`).
2. Empty line (after trim) → new prompt, nothing runs.
3. Leave raw mode.
4. Push into history when policy allows.
5. Execute the line; if the `exit` builtin ran, leave the shell; otherwise show a new prompt.

## Redraw

Each visible change repaints the current row as `prompt + input_line`, clears leftover characters to the end of the line, then places the caret at `prompt.len() + cursor_pos`.
