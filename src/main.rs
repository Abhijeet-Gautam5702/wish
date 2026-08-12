use std::{
    env, eprintln,
    io::{Write, stdin, stdout},
    path::Path,
    print, println,
    process::{Child, ChildStdout, Command, Stdio},
};

/// PIPED COMMANDS
///
/// Terminal STDIN --> Command-1 ---Pipe--> Command-2 ---Pipe---> ... ---Pipe--> Command-N --> TERMINAL STDOUT
///
/// 1. Parse the command string into pipeline stages
/// 2. Configure STDIN and STDOUT for each stage
/// 3. Spawn each stage concurrently and wait for them to finish
/// 4. Move to next user prompt ('> ')
///
/// Failure Policies
///
/// 1. Process Spawn Failure
/// - Print the error
/// - Clear all STDIN/STDOUT of all the stages
/// - Wait for the already-started child processes
/// - Exit this pipeline and prompt the user for next command
///
/// 2. Process Exit Failure (Spawned successfully but exited due to failure)
/// - Wait for all child processes to complete
/// - Treat the end-command's exit status as the final status
/// - Prompt the user for next command

fn main() {
    let mut user_inputs: Vec<String> = vec![];
    loop {
        let current_path = env::current_dir().unwrap();
        print!("{} $ ", current_path.to_str().unwrap());
        // Rust doesn't write '> ' to the terminal immediately
        // because Rust keeps the stdout prints in buffer for efficiency
        // To trigger a flush, either there should be a newline (use println!)
        // or you have to explicitly flush to the terminal/stdout
        stdout().flush().unwrap(); // flush the '> ' to stdout explicitly

        let mut user_input = String::new();
        stdin().read_line(&mut user_input).unwrap();
        user_inputs.push(user_input.clone());

        let mut pipeline_stages = user_input.split(" | ").map(|x| x.trim()).peekable();
        let mut previous_stdout: Option<ChildStdout> = None;
        let mut child_processes: Vec<Child> = vec![];
        let mut pipeline_failed = false;

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
                        println!("{}", e);
                    }
                }
                // Exit the shell program
                "exit" => return,
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
                            pipeline_failed = true;
                            drop(previous_stdout.take());
                            break;
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
                    eprintln!("Failed to wait for the child: {}", wait_err);
                }
            }
        }

        // Continue with the next command prompt
        if pipeline_failed {
            continue;
        }
    }
}
