use std::os::unix::net::UnixListener;
use std::io::{self, BufRead, BufReader, Write};

pub struct IpcServer {
    listener: UnixListener,
}

impl IpcServer {
    pub fn new(socket_path: &str) -> io::Result<Self> {
        std::fs::remove_file(socket_path).ok();
        let listener = UnixListener::bind(socket_path)?;
        Ok(Self { listener })
    }

    pub fn start<F>(&self, handle_request: F) -> io::Result<()>
    where
        F: Fn(String) -> String,
    {
        for stream in self.listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    println!("Connection established.");
                    let mut reader = BufReader::new(&stream);
                    let mut buffer = String::new();
                    reader.read_line(&mut buffer)?;

                    let response = handle_request(buffer.trim().to_string());
                    if let Err(e) = stream.write_all(response.as_bytes()) {
                        eprintln!("Error writing response: {:?}", e);
                    }
                }
                Err(e) => eprintln!("Error accepting connection: {:?}", e),
            }
        }
        Ok(())
    }
}
