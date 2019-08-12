use std::process::{Command, Child, Stdio, ChildStdout, ChildStdin};
use std::sync::mpsc;
use std::thread;
use std::io::{BufReader, Write, Read, BufRead};

pub struct RunningProcess {
    process: Child,
    pub recive_out: mpsc::Receiver<String>,
    handle_output: thread::JoinHandle<()>,
    pub send_in: mpsc::Sender<String>,
    handle_input: thread::JoinHandle<()>,
}

impl RunningProcess{

    /* Launch the specified process
     * TODO: manage the std err
     */
    pub fn new(path: String) -> Result<RunningProcess, Box<std::error::Error>>{
        let p = Command::new(path) // Launchin the process
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .spawn();

        let mut child = p?; // std::io::Error
        /* Passing the stdout of the process to a thread whose sole purpose is to gather it and send it in the channel that it was given
         * This is done because the output has a limited buffer and, upon overflow, it will cause a crash of the application
         * If for any reason the stdout is also needed elsewhere, it can always be implemented in the struct RunninProcess and borrowed from there
         * */
        let stdout = child.stdout.take().unwrap(); // TODO: unwrap no good
        let (out_send, out_recieve) = mpsc::channel();
        let handle_out = thread::spawn(move || {
            gather_output(stdout, out_send);
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
            recive_out: out_recieve,
            handle_output: handle_out,
            send_in: in_send,
            handle_input: handle_in,
        })
    }// new

}// impl RunninProcess

/* Collect all the stdout of a process and place it in the channel sender it has recived
 * It's meant to be executed by a thread
 */
fn gather_output(stdout: ChildStdout, out_send: mpsc::Sender<String>){ 
    let buff = BufReader::new(stdout);
    let lines = buff.lines();
    for line in lines { // Catching all lines
        match line { // Managing if there was a problem reading a line
            Err(_) => (), // TODO: manage this possible error

            Ok(l) => {
                match out_send.send(l) { // Managing if coudnt send a line
                    Err(e) => (), // TODO: manage this possible error

                    Ok(_) => (), // Success
                }
            },
        }
    }; // end for
}

/* Recieve the stdin of the process and feeds it into the process
 * It's meant to be executed by a thread
 */ // TODO:
fn send_input(mut stdin: ChildStdin, in_recieve: mpsc::Receiver<String>) {
    for recieved in in_recieve {
        match stdin.write_all(recieved.as_bytes()){ // TODO: need to manages errors
            Err(_) => (), 
            Ok(_) => (),
        };
    };
}

/* Tests
fn test{
    let rn = running_process::RunningProcess::new(String::from("./parrot")).unwrap();
    // Spawning a thread that output the stdout
    let reciever = rn.recive_out;
    std::thread::spawn(move || {     
        println!("---> Recieving <---");
        for recived in reciever{
            println!("Recived: {}", recived);
        };
    });
    // Sending some input
    println!("---> Sending <---");
    let lines = BufReader::new(std::io::stdin()).lines();
    for line in lines {
        match line {
            Err(_) => (),
            Ok(l) => {
                if l == String::from("stop"){
                    break;
                };
                let message = format!("{}\r", l);
                match rn.send_in.send(message) {
                    Err(_) => (),
                    Ok(_) => (),
                };
                match rn.send_in.send(String::from("\n")) {
                    Err(_) => (),
                    Ok(_) => (),
                };
            },
        };
    };
}*/