use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

/// Struct containing the necessary to manage the process
pub struct RunningProcess {
    process: Child,
    /// Pipe that recieve the process stdout
    pub stdout: Option<mpsc::Receiver<String>>,
    handle_output: Option<thread::JoinHandle<()>>,
    /// Pipe that recieve the process stderr
    pub stderr: Option<mpsc::Receiver<String>>,
    handle_error: Option<thread::JoinHandle<()>>,
    ///
    pub stdin: Option<mpsc::Sender<String>>,
    handle_input: Option<thread::JoinHandle<()>>,
    /// shared state about the current state of the process life
    is_alive: Arc<Mutex<bool>>,
}

impl RunningProcess {
    /* Launch the specified process
     * args are optional thats why they are an option
     * TODO: manage the std err
     * TODO: description
     */
    pub fn new(
        path: String,
        args: Option<Vec<String>>,
    ) -> Result<RunningProcess, Box<dyn std::error::Error>> {
        // Creating the setup for the process
        let mut setup_p = Command::new(path);

        // Setup stdio
        setup_p
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped());

        // Add args if present
        if let Some(x) = args {
            setup_p.args(x);
        }

        // startiung the process
        let p = setup_p.spawn();

        let mut child = match p {
            Err(e) => return Err(Box::new(e)), // TODO: return Error, not panic
            Ok(p) => p,
        }; // returned error: std::io::Error
           /* Passing the stdout of the process to a thread whose sole purpose is to gather it and send it in the channel that it was given
            * This is done because the output has a limited buffer and, upon overflow, it will cause a crash of the application
            * If for any reason the stdout is also needed elsewhere, it can always be implemented in the struct RunninProcess and borrowed from there
            * In case te process hasn't returned stdout, just return none TODO: should be an error
            * */
        let (out_recieve, handle_out) = match child.stdout.take() {
            None => (None, None), // TODO: return Error, not None
            Some(stdout) => {
                let (out_send, out_recieve) = mpsc::channel();
                let handle_out = thread::spawn(move || {
                    gather_output(stdout, out_send);
                });
                (Some(out_recieve), Some(handle_out))
            }
        };
        /* Passing the stderr of the process to a thread whose sole purpose is to gather it and send it in the channel that it was given
         * This is done because the stderr has a limited buffer and, upon overflow, it will cause a crash of the application
         * If for any reason the stderr is also needed elsewhere, it can always be implemented in the struct RunninProcess and borrowed from there
         * In case te process hasn't returned stdout, just return none TODO: should be an error
         * */
        let (err_recieve, handle_err) = match child.stderr.take() {
            None => (None, None), // TODO: return Error, not None
            Some(stderr) => {
                let (err_send, err_recieve) = mpsc::channel();
                let handle_err = thread::spawn(move || {
                    gather_output(stderr, err_send);
                });
                (Some(err_recieve), Some(handle_err))
            }
        };
        /* Passing the stdin of the process to a thread whose sole purpose is to recive the stdin from a channel that it was given
         * This is done to make sure it's always possible for the process to recive the input it needs
         * If for any reason the stdin is also needed elsewhere, it can always be implemented in the struct RunninProcess and borrowed from there
         * In case te process hasn't returned stdout, just return none TODO: should be an error
         * */
        let is_alive = Arc::new(Mutex::new(true));
        let is_alive_link = Arc::clone(&is_alive);
        let (in_send, handle_in) = match child.stdin.take() {
            None => (None, None), // TODO: return Error, not panic
            Some(stdin) => {
                let (in_send, in_recieve) = mpsc::channel();
                let handle_in = thread::spawn(move || {
                    send_input(stdin, in_recieve, is_alive_link);
                });
                (Some(in_send), Some(handle_in))
            }
        };

        // TODO: implement a check that waits for the input and output trhead to be ready

        // Returning struct
        Ok(RunningProcess {
            process: child,
            stdout: out_recieve,
            handle_output: handle_out,
            stderr: err_recieve,
            handle_error: handle_err,
            stdin: in_send,
            handle_input: handle_in,
            is_alive: is_alive,
        })
    } // new

    /// Kills the process if it's still running
    pub fn kill(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: manage lock
        let mut is_alive = self.is_alive.lock().unwrap();
        *is_alive = false;
        Ok(self.process.kill()?)
    }

    /// Return the OS-identifier associated with this child
    pub fn id(&self) -> u32 {
        self.process.id()
    }

    /// Attempt to return the exit status of the child if it has already exited
    pub fn is_alive(&mut self) -> Result<Option<ExitStatus>, Box<dyn std::error::Error>> {
        match self.process.try_wait() {
            Err(e) => panic!(Box::new(e)), //TODO: bad panic
            Ok(some) => Ok(some),
        }
    }

    /// Wait for the process to end, return the exit code
    pub fn wait(&mut self) -> std::io::Result<ExitStatus> {
        let result = self.process.wait();
        // Need to update is alive for the stdin
        // TODO: bad panic
        match self.is_alive.lock() {
            Err(e) => panic!("Can't lock: {}", e),
            Ok(mut guard) => *guard = false,
        };
        result
    }
} // impl RunninProcess

/// Collect all the stdout/stderr of a process and place it in the channel sender it has recived
/// It's meant to be executed by a thread
// TODO: mange errors
fn gather_output<T: Read>(std: T, pipe_send: mpsc::Sender<String>) {
    for line in BufReader::new(std).lines() {
        // Catching all lines
        match line {
            // Managing if there was a problem reading a line
            Err(_) => (), // TODO: manage this possible error

            Ok(l) => {
                match pipe_send.send(l) {
                    // Managing if coudnt send a line
                    // For now we will assume that if there is an error is because the pipe was closed
                    Err(_) => break, // TODO: manage this possible error, if it doesn't work, the pipe is closed? https://doc.rust-lang.org/std/sync/mpsc/struct.RecvError.html
                    Ok(_) => (),     // Success
                }
            }
        }
    } // end for
    println!("closed reciever");
}

/// Recieve the stdin of the process and feeds it into the process
/// It's meant to be executed by a thread
fn send_input(
    mut stdin: ChildStdin,
    in_recieve: mpsc::Receiver<String>,
    is_alive: Arc<Mutex<bool>>,
) {
    loop {
        match in_recieve.try_recv() {
            Ok(recieved) => {
                match stdin.write_all(recieved.as_bytes()) {
                    // TODO: need to manages errors
                    Err(_) => (),
                    Ok(_) => (),
                };
            }
            Err(_) => (), // TODO: manage errors
        }
        match is_alive.try_lock() {
            Err(_) => (),
            Ok(is_alive) => {
                if !*is_alive {
                    break;
                }
            }
        }
    }
}

// Implementing what happen when the struct is removed from the memory
impl Drop for RunningProcess {
    fn drop(&mut self) {
        // If they exist, close the pipes
        match self.stdin.take() {
            Some(pipe) => drop(pipe),
            None => println!("no input pipe to close"),
        }
        match self.stdout.take() {
            Some(pipe) => drop(pipe),
            None => (),
        }
        match self.stderr.take() {
            Some(pipe) => drop(pipe),
            None => (),
        }
        // The guard will release the access only when it goes out of scope
        // This code is inside a block because we need to free the gurad
        {
            let mut is_alive = self.is_alive.lock().unwrap();
            *is_alive = false;
        }
        // Use the handles to wait for the end of each thread
        match self.handle_input.take() {
            Some(handle) => {
                println!("Joining input");
                match handle.join() {
                    Ok(_) => println!("Input thread stopped"),
                    Err(_) => println!("Could not join the input thread"),
                };
            }
            None => (),
        }
        match self.handle_output.take() {
            Some(handle) => {
                println!("Joining output");
                match handle.join() {
                    Ok(_) => println!("Output thread stopped"),
                    Err(_) => println!("Could not join the output thread"),
                };
            }
            None => (),
        }
        match self.handle_error.take() {
            Some(handle) => {
                println!("Joining error");
                match handle.join() {
                    Ok(_) => println!("Input thread stopped"),
                    Err(_) => println!("Could not join the error thread"),
                };
            }
            None => (),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Tests
////////////////////////////////////////////////////////////////////////////////////////////////////
#[cfg(test)]
mod test {

    use super::*;

    // Test over a program that repeat int's input
    #[test]
    fn parrot_test() {
        let cmd = String::from("./parrot");
        let mut rn = RunningProcess::new(cmd, None).unwrap();
        // Words to feed to the parrot
        let mut parrot_words: Vec<String> = vec![String::from("hello"), String::from("ciao")];
        parrot_words.push(String::from("stop")); // key word to stop the parrot
                                                 // Sending some words to the parrot
        let parrot_ear = rn.stdin.take().unwrap();
        for word in parrot_words.clone() {
            match parrot_ear.send(format!("{}\r\n", word)) {
                Err(e) => println!("Sending error: {}", e),
                Ok(_) => (),
            };
        }
        // Recieving and checking the parrots output
        let mut i = 0;
        let parrot_voice = rn.stdout.take().unwrap();
        for recieved in parrot_voice {
            assert_eq!(recieved, parrot_words[i]);
            i = i + 1;
        }
    } // parrot_test

    #[test]
    fn echo_test() {
        // echo comands
        let cmd = String::from("echo");
        // echo args
        let mut args: Vec<String> = Vec::new();
        args.push(String::from("hello"));
        args.push(String::from("world"));
        let mut rn = RunningProcess::new(cmd, Some(args)).unwrap();
        let reciever = rn.stdout.take().unwrap();
        for recieved in reciever {
            assert_eq!(String::from("hello world"), recieved);
        }
    } // echo_test
} // Tests
