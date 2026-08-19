# Interactive line editing

How Wish reads a command line before execute. A `cursor_pos` tracks the caret; Left/Right move it (ASCII / byte-oriented for now).

## Overview

1. Enter raw mode for the prompt (`TerminalRawMode` guard).
2. Show `prompt` + `input_line` (CWD + ` $ `).
3. Handle keys one at a time; redraw when the buffer changes.
4. On Enter: leave raw (cooked), then run the line (see also [Command history](command-history.md)).

## Flow

```mermaid
flowchart TD
  START([Shell start]) --> PROMPT

  PROMPT[Show prompt] --> RAW[Enable raw mode]
  RAW --> RESET[Clear input_line / draft state]
  RESET --> READ[Read key event]

  READ --> MATCH{Key?}

  MATCH -->|Char printable| CHAR[Append + redraw]
  MATCH -->|Backspace| BS[Pop + redraw]
  MATCH -->|Up / Down| HIST[History browse]
  MATCH -->|Enter| ENTER[Submit path]
  MATCH -->|Esc| EXIT([Exit shell])
  MATCH -->|Other| NOOP[No-op]
  NOOP --> READ

  CHAR --> READ
  BS --> READ
  HIST --> READ

  ENTER --> COOKED[Cooked mode]
  COOKED --> EXEC[Push history if any + execute]
  EXEC --> EXITCHK{exit?}
  EXITCHK -->|yes| DONE([Exit])
  EXITCHK -->|no| PROMPT
```

History Up/Down details: [Command history](command-history.md).

## Terminal mode (raw / cooked)

| Case | Action |
|------|--------|
| Start of prompt | Enable raw via `TerminalRawMode` guard |
| Guard drops | Disable raw (normal end of scope or panic unwind) |
| Before execute | Terminal must be cooked so children behave |
| `exit` / Esc | Leave shell; raw restored by drop / process end |

**Rules**

- Raw only while collecting the line.
- Prefer a RAII guard over scattered enable/disable calls.
- Manual cooked before execute is fine if the guard is still alive until end of the prompt iteration.

## Keystroke loop

Per keystroke: `match` on `KeyEvent` → update `input_line` (and related state) → redraw when needed.

Not: `read_line` and only act after Enter.

| Input | Action |
|-------|--------|
| Printable `Char(c)` without Ctrl (`!c.is_control()`) | Insert at `cursor_pos`, advance cursor, redraw |
| Ctrl+C | Cancel line → new prompt (`break` key loop) |
| Ctrl+D (empty line) | Exit shell |
| Ctrl+D (non-empty) / other Ctrl+key | No-op |
| Backspace | Delete character before cursor (if any), redraw |
| Left / Right | Move `cursor_pos` within `[0, len]`; no-op at ends |
| Enter | Submit (see below) |
| Esc | Exit shell |
| Up / Down | History (see [Command history](command-history.md)); cursor jumps to end of loaded line |

## Backspace

| Case | Action |
|------|--------|
| Empty `input_line` | No-op (do not erase into the prompt) |
| Non-empty | Pop one character, redraw |

## Enter / submit

```text
1. Write \r\n (move to next screen line)
2. If input_line empty (trim) → new prompt, no execute
3. Cooked mode
4. Push history if policy allows (see [Command history](command-history.md))
5. execute_command(input_line)
6. If exit builtin → leave shell; else next prompt
```

| Case | Action |
|------|--------|
| Empty line | No push, no execute, re-prompt |
| Non-empty | History push (policy), then execute |
| `exit` | Shell exits |

## Redraw

Repaint the current row as `prompt + input_line`, then place the caret.

| Case | Action |
|------|--------|
| After Char / Backspace / Left / Right / history load | Redraw |
| Line got shorter | Clear to end of line so old characters do not linger |
| Empty buffer | Show prompt only |
| Caret | Column ≈ `prompt.len() + cursor_pos` (ASCII / byte-oriented) |
| After Up/Down | `cursor_pos = input_line.len()` (end of line) |

Typical approach: move to column 0 → clear until end of line → write prompt + buffer → `MoveToColumn` → flush.
