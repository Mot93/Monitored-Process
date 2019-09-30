use std::process::{Command, Child, Stdio, ChildStdin, ExitStatus};
use std::sync::mpsc;
use std::thread;
use std::io::{BufReader, Write, Read, BufRead};

/// Struct containing the necessary to manage the 
pub struct RunningProcess {
    process: Child,
    pub recieve_out: mpsc::Receiver<String>,
    handle_output: thread::JoinHandle<()>,
    pub recieve_err: mpsc::Receiver<String>,
    handle_error: thread::JoinHandle<()>,
    pub send_in: mpsc::Sender<String>,
    handle_input: thread::JoinHandle<()>,
}

impl RunningProcess{

    /* Launch the specified process
     * args are optional thats why they are an option
     * TODO: manage the std err
     */
    pub fn new(path: String, args: Option<Vec<String>>) -> Result<RunningProcess, Box<dyn std::error::Error>>{
        // Creating the setup for the process
        let mut setup_p = Command::new(path);

        // Setup stdio
        setup_p.stdout(Stdio::piped())
            .stdin(Stdio::piped());

        // Add args if present
        if let Some(x) = args {
            setup_p.args(x);
        }
            
        // startiung the process
        let p = setup_p.spawn();

        let mut child = p?; // returned error: std::io::Error
        /* Passing the stdout of the process to a thread whose sole purpose is to gather it and send it in the channel that it was given
         * This is done because the output has a limited buffer and, upon overflow, it will cause a crash of the application
         * If for any reason the stdout is also needed elsewhere, it can always be implemented in the struct RunninProcess and borrowed from there
         * */
        let stdout = child.stdout.take().unwrap(); // TODO: unwrap no good
        let (out_send, out_recieve) = mpsc::channel();
        let handle_out = thread::spawn(move || {
            gather_output(stdout, out_send);
        });
        /* Passing the stderr of the process to a thread whose sole purpose is to gather it and send it in the channel that it was given
         * This is done because the stderr has a limited buffer and, upon overflow, it will cause a crash of the application
         * If for any reason the stderr is also needed elsewhere, it can always be implemented in the struct RunninProcess and borrowed from there
         * */
        let stderr = child.stderr.take().unwrap(); // TODO: unwrap no good
        let (err_send, err_recieve) = mpsc::channel();
        let handle_err = thread::spawn(move || {
            gather_output(stderr, err_send);
        });
        /* Passing the stdin of the process to a thread whose sole purpose is to recive the stdin from a channel that it was given
         * This is done to make sure it's always possible for the process to recive the input it needs
         * If for any reason the stdin is also needed elsewhere, it can always be implemented in the struct RunninProcess and borrowed from there
         * */
        let stdin = child.stdin.take().unwrap(); // TODO: unwrap no good
        let (in_send, in_recieve) = mpsc::channel();
        let handle_in = thread::spawn(move || {
            send_input(stdin, in_recieve);
        });
        // Secondary struct

        // Returning struct
        Ok(RunningProcess{
            process: child,
            recieve_out: out_recieve,
            handle_output: handle_out,
            recieve_err: err_recieve,
            handle_error: handle_err,
            send_in: in_send,
            handle_input: handle_in,
        })
    }// new

    /// Kills the process if it's still running
    pub fn kill(&mut self) -> Result<(), Box<std::error::Error>> {
        Ok(self.process.kill()?)
    }

    /// Return the OS-identifier associated with this child
    pub fn id(&self) -> u32{
        self.process.id()
    }

    /// Attempt to return the exit status of the child if it has already exited
    pub fn is_alive(&mut self) -> Result<Option<ExitStatus>, Box<std::error::Error>> {
        match self.process.try_wait() {
            Err(e) => Err(Box::new(e)),
            Ok(some) => Ok(some),
        }
    }

}// impl RunninProcess

/// Collect all the stdout/stderr of a process and place it in the channel sender it has recived
/// It's meant to be executed by a thread
// TODO: mange errors
fn gather_output<T: Read>(std: T, pipe_send: mpsc::Sender<String>){ 
    for line in BufReader::new(std).lines() { // Catching all lines
        match line { // Managing if there was a problem reading a line
            Err(_) => (), // TODO: manage this possible error

            Ok(l) => {
                match out_send.send(l) { // Managing if coudnt send a line
                    Err(_) => (), // TODO: manage this possible error

                    Ok(_) => (), // Success
                }
            },
        }
    }; // end for
}

/// Recieve the stdin of the process and feeds it into the process
/// It's meant to be executed by a thread
fn send_input(mut stdin: ChildStdin, in_recieve: mpsc::Receiver<String>) {
    for recieved in in_recieve {
        match stdin.write_all(recieved.as_bytes()){ // TODO: need to manages errors
            Err(_) => (), 
            Ok(_) => (),
        };
    };
}

// Tests
#[cfg(test)]
mod test{

    use super::*;

    #[test]
    fn parrot_test(){
        let cmd = String::from("./parrot");
        let rn = RunningProcess::new(cmd, None).unwrap();
        // Words to feed to the parrot
        let mut parrot_words: Vec<String> = vec![String::from("hello"), String::from("ciao")];
        parrot_words.push(String::from("stop")); // key word to stop the parrot
        // Sending some words to the parrot
        for word in parrot_words.clone(){
            match rn.send_in.send(format!("{}\r\n", word)){
                Err(e) => println!("Sending error: {}", e),
                Ok(_) => (),
            };
        }; 
        // Recieving and checking the parrots output
        let mut i = 0;
        for recieved in rn.recive_out{
            assert_eq!(format!("{}", recieved), format!("Readed: {}", parrot_words[i]));
            i = i + 1;
        };
    }// parrot_test 

    #[test]
    fn echo_test(){
        // echo comands
        let cmd = String::from("echo");
        // echo args
        let mut args: Vec<String> = Vec::new();
        args.push(String::from("hello"));
        args.push(String::from("world"));
        let rn = RunningProcess::new(cmd, Some(args)).unwrap();
        let reciever = rn.recive_out;
        for recieved in reciever {
            assert_eq!(String::from("hello world"), recieved);
        }
    }// echo_test

}// Tests
