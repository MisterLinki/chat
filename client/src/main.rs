use std::{io::{self, prelude::*}, net::TcpStream, thread, time};
use chrono::Local;

fn main(){
    print!("enter ip address server: ");
    let mut ip = String::new();
    let _ = io::stdout().flush();
    io::stdin().read_line(&mut ip).unwrap();

    loop {
        match TcpStream::connect(format!("{}:4545", ip.trim_end())) {
            Ok(mut stream) => {
                let mut stream_cloned = stream.try_clone().unwrap();
    
                thread::spawn( move || {
                    loop {
                        let mut buffer = [0; 1024];
                        let bytes = stream.read(&mut buffer).unwrap();
                        println!("\n{}", String::from_utf8_lossy(&buffer[..bytes]).trim_end().to_owned());
                    }
                });

                loop {
                    thread::sleep(time::Duration::from_millis(500));
                    print!("[{} : You] : ", Local::now().format("%d/%m/%Y %H:%M"));
                    let mut text: String = String::new();
                    let _ = io::stdout().flush();
                    io::stdin().read_line(&mut text).unwrap();
                    
                    stream_cloned.write_all(text.trim_end().as_bytes()).unwrap();
                }
            },
            Err(_) => {},
        };
    }
}