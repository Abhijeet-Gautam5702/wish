use crossterm::{
    cursor,
    event::{Event, KeyCode, KeyEvent, KeyModifiers, read},
    execute,
    terminal::{self, Clear},
};
use std::{
    cmp, env, eprintln, format,
    io::{self, Error, Write, stdout},
    path::Path,
    process::{Child, ChildStdout, Command, Stdio},
    write,
};

const MAX_HISTORY_ITEMS: usize = 10;

/// Whether the line editor is on a fresh draft or browsing history.
#[derive(PartialEq, Eq)]
enum ShellMode {
    /// Viewing / editing a line loaded from history.
    History,
    /// Typing a new line (or restored draft after Down past newest).
    Draft,
}

/// RAII guard: raw mode while this value is alive; cooked again on drop.
struct TerminalRawMode;
impl TerminalRawMode {
    /// Enable terminal raw mode and return a guard that disables it when dropped.
    fn enter() -> Result<Self, io::Error> {
        terminal::enable_raw_mode()?;
        Ok(Self)
    }
}
impl Drop for TerminalRawMode {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
    }
}

/// Repaint the current prompt line as `prompt` + `input_line`
/// at a given cursor position
fn redraw(prompt: &str, input_line: &str, cursor_pos: usize) -> Result<(), io::Error> {
    let mut stdout = stdout();
    execute!(
        stdout,
        cursor::MoveToColumn(0),
        Clear(terminal::ClearType::UntilNewLine)
    )?;
    write!(stdout, "{}{}", prompt, input_line)?;
    stdout.flush()?;
    let effective_cursor_pos = prompt.len() + cursor_pos;
    execute!(stdout, cursor::MoveToColumn(effective_cursor_pos as u16))?; // move the cursor to the desired position
    Ok(())
}

/// Run a line: builtins (`cd`, `exit`) or an external pipeline (`cmd | cmd | ...`).
/// Returns `Ok(true)` if the shell should exit, `Ok(false)` to keep prompting.
fn execute_command(command_string: &mut String) -> Result<bool, io::Error> {
    let mut pipeline_stages = command_string.split(" | ").map(|x| x.trim()).peekable();
    let mut previous_stdout: Option<ChildStdout> = None;
    let mut child_processes: Vec<Child> = vec![];

    while let Some(stage) = pipeline_stages.next() {
        let mut fragments_iterator = stage.trim().split_whitespace();
        let command = fragments_iterator.next().unwrap();
        let arguments = fragments_iterator; // iterator of &str

        match command {
            // Shell built-ins
            "cd" => {
                // .peek() adds an extra reference
                // arguments is already an iterator of &str
                // .peek() converts it to an iterator of &&str
                // that is why we dereference it using .map_or(|x| *x)
                // so that new_dir becomes &str again
                let new_dir = arguments.peekable().peek().map_or("/", |x| *x);
                let root = Path::new(new_dir);
                if let Err(e) = env::set_current_dir(root) {
                    eprintln!("cd commmand failed: {}", e);
                    continue;
                }
            }
            // Exit the shell program completely
            "exit" => {
                return Ok(true);
            }
            _ => {
                // Configure STDIN/STDOUT handles
                let stdin = previous_stdout
                    .take() // Take the value out of Option<T>
                    // Assign shell's file-descriptor if this is first stage
                    // otherwise create a new handle from the previous_stdout.take()
                    .map_or_else(Stdio::inherit, Stdio::from);
                let stdout = if pipeline_stages.peek().is_some() {
                    // If this is a middle-stage (there is a stage after this)
                    // current child's stdout will be an OS pipe
                    Stdio::piped()
                } else {
                    // If this is the last stage (no further command after this)
                    // current child's stdout will be the shell's stdout (terminal)
                    Stdio::inherit()
                };

                // Creates a command configuration
                // adds arguments to it
                // asks the OS to create (spawn) and start the child process
                // .spawn() returns the Child handle as soon as the process starts execution
                // (and not after it has executed)
                let child_process = Command::new(command)
                    .args(arguments)
                    .stdin(stdin) // Provide the STDIN Handler for this child command
                    .stdout(stdout) // Provide the STDOUT Handler for this child command
                    .spawn();

                match child_process {
                    Ok(mut child) => {
                        // Set the previous_stdout for the next stage processing
                        // This previous_stdout will becomes stdin for the next stage
                        previous_stdout = child.stdout.take();
                        child_processes.push(child);
                    }
                    Err(e) => {
                        eprintln!("Process Spawn Failed: {}", e);
                        // Clear any unused handles owned by shell
                        // Stop constructing the pipeline further
                        drop(previous_stdout.take());
                        break; // don't process further
                    }
                }
            }
        }
    }

    // Wait for the pipeline to complete
    // Pipeline completion === Each stage in the pipeline completes
    for child in &mut child_processes {
        match child.wait() {
            Ok(_exit_status) => {}
            Err(wait_err) => {
                let error_msg = format!("Failed to wait for the child: {}", wait_err);
                return Err(Error::new(io::ErrorKind::Other, error_msg));
            }
        }
    }
    Ok(false)
}

/// Push a submitted command into history.
/// Skips if it matches the last entry; drops the oldest when at MAX capacity.
fn history_push(history: &mut Vec<String>, input_line: &String) {
    let curr_history_len = history.len();
    // skip if last entry is same as new entry
    if curr_history_len >= 1 {
        let last = history.last().unwrap();
        if last == input_line {
            return;
        }
    }
    if curr_history_len == MAX_HISTORY_ITEMS {
        history.remove(0);
    }
    history.push(input_line.clone());
}

fn run_shell() -> Result<(), io::Error> {
    let mut history: Vec<String> = vec![];
    // Outer Shell Running Loop
    loop {
        let _raw_mode = TerminalRawMode::enter()?;

        let current_path = env::current_dir().unwrap();
        let prompt = format!("{} $ ", current_path.to_str().unwrap());

        let mut input_line: String = String::from("");
        let mut draft_line: String = String::from("");
        let mut history_ptr_pos: usize = history.len();
        let mut cursor_pos: usize = 0;

        let mut shell_mode: ShellMode = ShellMode::Draft;

        redraw(&prompt, &input_line, cursor_pos)?;

        // Key-parsing loop (Reads keystrokes)
        loop {
            match read()? {
                Event::Key(KeyEvent {
                    code: KeyCode::Backspace,
                    ..
                }) => {
                    if cursor_pos > 0 {
                        cursor_pos -= 1;
                        input_line.remove(cursor_pos);
                        redraw(&prompt, &input_line, cursor_pos)?;
                    }
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Enter,
                    ..
                }) => {
                    // Move to the col-0 of next line to display command execution results
                    // "\r" -> move to col-0 of this line
                    // "\n" -> move to next line
                    write!(stdout(), "\r\n")?;

                    // No command input found
                    // move to next command prompt
                    if input_line.trim().is_empty() {
                        break;
                    }

                    // Bring the terminal into the cooked mode before executing the command
                    drop(_raw_mode);
                    // update command history
                    history_push(&mut history, &input_line);
                    let should_exit = execute_command(&mut input_line)?;
                    if should_exit {
                        return Ok(());
                    }
                    break; // break out from the key-parsing loop (to go to the next command prompt)
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Up, ..
                }) => {
                    if history.len() == 0 {
                        continue;
                    }
                    // No-op: Oldest history entry reached
                    if history_ptr_pos == 0 {
                        continue;
                    }
                    // Enter the History Mode
                    if shell_mode == ShellMode::Draft {
                        draft_line = input_line.clone();
                        shell_mode = ShellMode::History;
                    }
                    // Move up the history
                    input_line = history[history_ptr_pos - 1].clone();
                    history_ptr_pos -= 1;
                    cursor_pos = input_line.len();
                    redraw(&prompt, &input_line, cursor_pos)?;
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Down,
                    ..
                }) => {
                    if history.len() == 0 {
                        continue;
                    }
                    // No-op: Already in draft mode
                    if shell_mode == ShellMode::Draft {
                        continue;
                    }
                    // newest history entry reached => move to draft_line
                    else if history_ptr_pos == history.len() - 1 {
                        input_line = draft_line.clone();
                        draft_line = String::from("");
                        history_ptr_pos += 1;
                        shell_mode = ShellMode::Draft;
                    }
                    // Move down the history
                    else if history_ptr_pos < history.len() - 1 {
                        input_line = history[history_ptr_pos + 1].clone();
                        history_ptr_pos += 1;
                    }
                    cursor_pos = input_line.len();
                    redraw(&prompt, &input_line, cursor_pos)?;
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Left,
                    ..
                }) => {
                    if cursor_pos > 0 {
                        cursor_pos -= 1;
                        redraw(&prompt, &input_line, cursor_pos)?;
                    }
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Right,
                    ..
                }) => {
                    if cursor_pos < input_line.len() {
                        cursor_pos += 1;
                        redraw(&prompt, &input_line, cursor_pos)?;
                    }
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Esc, ..
                }) => {
                    // Exit the shell
                    return Ok(());
                }
                // Check if the keystroke is a printable character
                // append to input_line if it is
                Event::Key(KeyEvent {
                    code: KeyCode::Char(ch),
                    modifiers,
                    ..
                }) => {
                    // all key combinations where at least one keystroke is Ctrl
                    // e.g., Ctrl+Shift+C, Ctrl+D, Ctrl+Alt+C, etc.
                    if modifiers.contains(KeyModifiers::CONTROL) {
                        // exit current prompt & move to the next command prompt
                        if ch.eq_ignore_ascii_case(&'c') {
                            break;
                        }
                        // exit the shell if no input
                        else if ch.eq_ignore_ascii_case(&'d') && input_line.is_empty() {
                            return Ok(());
                        }
                    }
                    // printable character found => update input_line
                    else if !ch.is_control() {
                        input_line.insert(cursor_pos, ch);
                        cursor_pos += 1;
                        redraw(&prompt, &input_line, cursor_pos)?;
                    }
                }
                // Mouse events, Other keys, etc.
                _ => { /* do nothing */ }
            }
        }
    }
}

fn main() {
    run_shell().unwrap();
}
