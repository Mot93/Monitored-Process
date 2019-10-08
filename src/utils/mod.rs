use std::sync::{Arc, Mutex, mpsc};
use std::io::{BufRead, BufReader, BufWriter, Write};

/// Receieve the stdin and a vec of Senders
/// Will redirect the stdin to all the Senders
pub fn stdin_pipes(stdin: std::io::Stdin, pipes: Arc<Mutex<Vec<mpsc::Sender<String>>>>){
    let input = BufReader::new(stdin);
    for line in input.lines() {
        match line {
            Err(e) => {
                println!("Could not read from keyboard, error: {}", e);
                break;
            },
            Ok(line) => {
                match pipes.lock() {
                    Err(e) => println!("Error getting the lock: {}", e),
                    Ok(mut vec) => {
                        // remember wich pipe don't work anymore and remove them later
                        let mut erase: Vec<usize> = Vec::new();
                        for i in 0..vec.len() {
                            match vec[i].send(line.clone()) {
                                Err(e) => {
                                    println!("Error sending in stdin pipe: {}", e);
                                    erase.push(i);
                                },
                                Ok(_) => println!("sent {}", line),
                            };
                        }
                        for i in erase {
                            vec.remove(i);
                        }
                    },
                };
            },
        }
    }
}// stdin_pipes

/// Given a sender and a reciever, pipe all the content from the reciever to the sender
/// When one of the two mpsc il closed, terminate
pub fn piped_mpsc(in_pipe: mpsc::Receiver<String>, out_pipe: mpsc::Sender<String>){
    loop {
        match in_pipe.try_recv() {
            Err(e) => {
                if e == mpsc::TryRecvError::Disconnected{
                    break;
                }
            },
            Ok(recieved) => {
                match out_pipe.send(recieved) {
                    Err(e) => {
                        println!("couldnt send in pipe: {}", e);
                        break;
                    },
                    Ok(_) => println!("sent!"),
                };
            },
        };
    }// loop
    println!("exit piped_mpsc");
}// piped_mpsc

/// Given the sdtoud/stderr and a Reciever, print the output of the reciever on the stdout/stderr
pub fn print_std<T: Write>(mut std: T, pipe: mpsc::Receiver<String>){
    loop{
        match pipe.try_recv() {
            Err(e) => {
                if e == mpsc::TryRecvError::Disconnected {
                    break;
                };
            },
            Ok(recieved) => {
                println!("Hello there -{}-", recieved);
                match writeln!(std, "{}", recieved) {
                    Err(e) => {
                        println!("could not write: {}", e);
                        break;
                    },
                    Ok(_) => println!("sent {}", recieved),
                }
            },
        };
    };
    println!("Stopped to print std");
}// print_std