use crate::MILLISEC_PAUSE;

use std::io::{BufRead, BufReader, Write};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Receieve the stdin and a vec of Senders
/// Will redirect the stdin to all the Senders
pub fn stdin_pipes(stdin: std::io::Stdin, pipes: Arc<Mutex<Vec<mpsc::Sender<String>>>>) {
    let input = BufReader::new(stdin);
    for line in input.lines() {
        match line {
            Err(e) => {
                println!("Could not read from keyboard, error: {}", e);
                break;
            }
            Ok(line) => {
                match pipes.lock() {
                    Err(e) => println!("Error getting the lock: {}", e),
                    Ok(mut vec) => {
                        // remember wich pipe don't work anymore and remove them later
                        let mut erase: Vec<usize> = Vec::new();
                        for i in 0..vec.len() {
                            match vec[i].send(line.clone()) {
                                Err(e) => {
                                    erase.push(i);
                                }
                                Ok(_) => (),
                            };
                        }
                        for i in erase {
                            vec.remove(i);
                        }
                    }
                };
            }
        }
    }
} // stdin_pipes

/// Given a sender and a reciever, pipe all the content from the reciever to the sender
/// When one of the two mpsc il closed, terminate
/// end_string is used in cases where final character have to be added such when it's necessary to add
pub fn piped_mpsc(
    recieve_pipe: mpsc::Receiver<String>,
    send_pipe: mpsc::Sender<String>,
    end_string: Option<String>,
) {
    let end_of_string: String = match end_string {
        Some(end) => end,
        None => String::new(),
    };
    loop {
        match recieve_pipe.try_recv() {
            Err(e) => {
                if e == mpsc::TryRecvError::Disconnected {
                    break;
                }
            }
            Ok(recieved) => {
                let to_send = format!("{}{}", recieved, end_of_string.clone());
                match send_pipe.send(to_send) {
                    Err(e) => {
                        println!("couldnt send in pipe: {}", e);
                        break;
                    }
                    Ok(_) => (),
                };
            }
        };
        // testing if the pipe is still alive
        match send_pipe.send("".to_string()) {
            Err(_) => break,
            Ok(_) => (),
        }
        // Don't want to overcumber the system with too many requests
        thread::sleep(Duration::from_millis(MILLISEC_PAUSE));
    } // loop
} // piped_mpsc

/// Given the sdtoud/stderr and a Reciever, print the output of the reciever on the stdout/stderr
pub fn print_std<T: Write>(mut std: T, pipe: mpsc::Receiver<String>) {
    loop {
        match pipe.try_recv() {
            Err(e) => {
                if e == mpsc::TryRecvError::Disconnected {
                    break;
                };
            }
            Ok(recieved) => match writeln!(std, "{}", recieved) {
                Err(e) => {
                    println!("could not write: {}", e);
                    break;
                }
                Ok(_) => (),
            },
        };
        // Don't want to overcumber the system with too many requests
        thread::sleep(Duration::from_millis(MILLISEC_PAUSE));
    }
} // print_std
