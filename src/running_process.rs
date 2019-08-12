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

    /// If the path is valid, launch the specified process and return RunningProcess to manage it
    //TODO: manage the std err
    pub fn new(path: String) -> Result<RunningProcess, Box<std::error::Error>>{
        let p = Command::new(path) // Launchin the process
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

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
                match pipe_send.send(l) { // Managing if coudnt send a line
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



/* ----------------------------------------------------------------------------------------------------
 * Testing this module functionality
 * 
 */
#[cfg(test)]
mod tests {

    use super::RunningProcess;

    /* This test uses an executable called parrot 
     * parrot return every word that its sended to it
     * every word that starts with 'a' or 'A' 
     * if \n is the only thing send to parrot, parrot will crash
     * when the word "stop" is sent to parrot, the process dies
     */
    #[test]
    fn parrot_test() {
        let mut rn = RunningProcess::new(String::from("./parrot")).unwrap();
        /* Sending trought stdin all my strings
         * At the same time I create the list of expected stdout and stderr
         * Reminder: all words that start with "a" or "A" are going in the stderr
         */
        let mut stderr: Vec<String> = Vec::new();
        let mut stdout: Vec<String> = Vec::new();
        for word in vec![
            String::from("I"), 
            String::from("am"), 
            String::from("parrot!")
        ] { // for loop
            // stderr
            if word[0..1] == String::from("a") || word[0..1] == String::from("A") {
                stderr.push(word.clone());
            } else { // stdout
                stdout.push(word.clone());
            };
            // Sending in the stdin the word
            let input = format!("{}\n", word); // TODO: document the fact that the \n is necessary for the stdin to read
            rn.send_in.send(input).unwrap();
        };
        // After sending all of the content to the parrot I stop it whith it's keyword
        rn.send_in.send(String::from("stop\n")).unwrap(); 
        // Recieving from the stdout of the parrot
        let mut from_stdout: Vec<String> = Vec::new();
        for recieved in &rn.recieve_out {
            from_stdout.push(recieved);
        };
        // Recieving from the stderr
        let mut from_stderr: Vec<String> = Vec::new();
        for recieved in &rn.recieve_err {
            from_stderr.push(recieved);
        };
        // Checking the stdout
        assert_eq!(
            stdout.len(), from_stdout.len(),
            "\nExpected stoud len = {}   gotten stdout len = {}", stdout.len(), from_stdout.len(),
        );
        for i in 0..stdout.len() {
            assert_eq!(
                stdout.get(i), from_stdout.get(i),
                "\nExpected stoud len = {}   gotten stdout len = {}", stdout.get(i).unwrap(), from_stdout.get(i).unwrap(),
            );
        };
        // Checking the stdout
        assert_eq!(
            stderr.len(), from_stderr.len(),
            "\nExpected stoud len = {}   gotten stdout len = {}", stderr.len(), from_stderr.len(),
        );
        for i in 0..stdout.len() {
            assert_eq!(
                stderr.get(i), from_stderr.get(i),
                "\nExpected stoud len = {}   gotten stdout len = {}", stderr.get(i).unwrap(), from_stderr.get(i).unwrap(),
            );
        };
        // Correctness of the kill
        assert_eq!(
            rn.kill().unwrap(), (),
            "\nFailed to kill the process"
        );
    }// parrot_test 
}// test module
