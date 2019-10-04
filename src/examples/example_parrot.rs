extern crate monitor;

use std::thread;
use std::io::{BufReader, BufRead};

use monitor::RunningProcess;

fn main() {
    println!("The parrot will repeat everything you tell him, enter 'stop' to interrupt");
    // Creating the pocess
    let cmd = String::from("./parrot");
    let mut rn = match RunningProcess::new(cmd, None) {
        Err(e) => panic!("Could not start the parrot: {}", e),
        Ok(running_process) => running_process, 
    };
    // Creating a thread that will print the output of the process
    let pipe_out = rn.stdout.take().unwrap();
    let _print_handle = thread::spawn(move || {
        for line in pipe_out{
            println!("{}", line);
        };
    });
    // Buffering input from keyboard
    let keyboard_input = BufReader::new(std::io::stdin());
    let pipe_in = rn.stdin.take().unwrap();
    let _read_handle = thread::spawn(move || {
        // Sending input from keyboard to the parrot
        for line in keyboard_input.lines() {
            match line {
                Err(e) => panic!("Error reading from keayboard: {}", e),
                Ok(string) => {
                    // Sending the input to the parrot
                    match pipe_in.send(format!("{}\r\n", string)) {
                        Err(e) => println!("Error sending to parrot: {}", e),
                        Ok(_) => (),
                    };
                },
            };// match       
        };// for
    });// input thread
    match rn.wait() {
        Err(e) => println!("Error: {}", e),
        Ok(exit_status) => println!("Exit status: {}", exit_status),
    };
}