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
                println!("readed {}", line);
                match pipes.lock() {
                    Err(e) => println!("Error getting the lock: {}", e),
                    Ok(mut vec) => {
                        println!("vec {}", vec.len());
                        // remember wich pipe don't work anymore and remove them later
                        let mut erase: Vec<usize> = Vec::new();
                        for i in 0..vec.len() {
                            match vec[i].send(line.clone()) {
                                Err(e) => {
                                    println!("Error sending in stdin pipe: {}", e);
                                    erase.push(i);
                                },
                                Ok(_) => println!("sent {}", i),
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

/// Given the sdtoud/stderr and a Reciever, print the output of the reciever on the stdout/stderr
pub fn print_std<T: Write>(mut std: BufWriter<T>, pipe: mpsc::Receiver<String>){
    loop{
        match pipe.try_recv() {
            Err(e) => {
                if e == mpsc::TryRecvError::Disconnected {
                    break;
                };
            },
            Ok(recieved) => {
                match std.write(recieved.as_bytes()) {
                    Err(e) => {
                        println!("could not write: {}", e);
                        break;
                    },
                    Ok(_) => (),
                }
            },
        };
    };
}// print_std