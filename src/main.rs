use crossterm::{
    cursor,
    event::{Event, KeyCode, read},
    execute,
    terminal::{self, Clear},
};
use std::{
    env, eprintln, format,
    io::{self, Error, Write, stdout},
    path::Path,
    process::{Child, ChildStdout, Command, Stdio},
    write,
};

const MAX_HISTORY_ITEMS: usize = 10;

#[derive(PartialEq, Eq)]
enum ShellMode {
    History,
    Draft,
}

fn redraw(prompt: &str, input_line: &str) -> Result<(), io::Error> {
    let mut stdout = stdout();
    execute!(
        stdout,
        cursor::MoveToColumn(0),
        Clear(terminal::ClearType::UntilNewLine)
    )?;
    write!(stdout, "{}{}", prompt, input_line)?;
    stdout.flush()?;
    Ok(())
}

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

/// pushes the user command input to the history of commands
/// and pops off the oldest entry if MAX_HISTORY_ITEMS is reached
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
        terminal::enable_raw_mode()?;

        let current_path = env::current_dir().unwrap();
        let prompt = format!("{} $ ", current_path.to_str().unwrap());

        let mut input_line: String = String::from("");
        let mut draft_line: String = String::from("");
        let mut history_ptr_pos: usize = history.len();

        let mut shell_mode: ShellMode = ShellMode::Draft;

        redraw(&prompt, &input_line)?;

        // Key-parsing loop (Reads keystrokes)
        loop {
            let event = read()?;
            if event == Event::Key(KeyCode::Backspace.into()) {
                input_line.pop();
                redraw(&prompt, &input_line)?;
            } else if event == Event::Key(KeyCode::Enter.into()) {
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
                terminal::disable_raw_mode()?;
                // update command history
                history_push(&mut history, &input_line);
                let should_exit = execute_command(&mut input_line)?;
                if should_exit {
                    return Ok(());
                }
                break; // break out from the key-parsing loop (to go to the next command prompt)
            } else if event == Event::Key(KeyCode::Up.into()) {
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
                redraw(&prompt, &input_line)?;
            } else if event == Event::Key(KeyCode::Down.into()) {
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
                redraw(&prompt, &input_line)?;
            } else if event == Event::Key(KeyCode::Esc.into()) {
                // Exit the shell
                return Ok(());
            } else {
                // TODO: PRINTABLE CHARACTER CHECK
                if let Event::Key(key_event) = event {
                    if let KeyCode::Char(c) = key_event.code {
                        input_line.push(c);
                        redraw(&prompt, &input_line)?;
                    }
                }
                // Check if the keystroke is a printable character
                // append to input_line if it is
            }
        }

        terminal::disable_raw_mode()?;
    }
}

fn cleanup() {
    if terminal::is_raw_mode_enabled().unwrap() {
        terminal::disable_raw_mode().unwrap();
    }
}

fn main() {
    match run_shell() {
        Ok(_) => {}
        Err(_) => {}
    }
    cleanup();
}
