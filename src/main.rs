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
    let reciever = rn.recive_out;
    std::thread::spawn(move || {     
        for recived in reciever{
            println!("Recived: {}", recived);
        };
    });
    // Sending some input
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
}