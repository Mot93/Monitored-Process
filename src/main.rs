extern crate chrono;

use std::io::{BufRead, BufReader};

mod running_process;

fn main() {
    run();
    println!("----------> The end");
}

fn run() {
    let rn = running_process::RunningProcess::new(String::from("./parrot")).unwrap();
    // Spawning a thread that output the stdout
    let reciever = rn.recieve_out;
    let handle = std::thread::spawn(move || {     
        for recived in reciever{
            println!("---> {}", recived);
        };
        println!("Thread is dead");
    });
    // Sending some input
    let lines = BufReader::new(std::io::stdin()).lines();
    println!("Write what the parrot has to repeat");
    for line in lines {
        match line {
            Err(_) => (),
            Ok(l) => {
                if l == String::from("stop"){
                    rn.send_in.send(String::from("stop\n")).unwrap();
                    break;
                };
                let message = format!("{}\n", l);
                rn.send_in.send(message).unwrap();
            },
        };
    };
    println!("Waiting for the thread to die");
    handle.join().unwrap();
} 