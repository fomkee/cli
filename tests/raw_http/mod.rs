#![cfg(test)]
use std::io::{self, BufRead, BufReader, Read};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

pub fn accept(listener: &TcpListener) -> io::Result<TcpStream> {
    let started = Instant::now();
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                stream.set_read_timeout(Some(Duration::from_secs(5)))?;
                stream.set_write_timeout(Some(Duration::from_secs(5)))?;
                return Ok(stream);
            }
            Err(error)
                if error.kind() == io::ErrorKind::WouldBlock
                    && started.elapsed() < Duration::from_secs(5) =>
            {
                thread::sleep(Duration::from_millis(5))
            }
            Err(error) => return Err(error),
        }
    }
}

pub fn read_request(stream: &mut TcpStream) -> io::Result<()> {
    let mut reader = BufReader::new(stream);
    let mut length = 0;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 || line == "\r\n" {
            break;
        }
        if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
            length = value.trim().parse().map_err(io::Error::other)?;
        }
    }
    reader.read_exact(&mut vec![0; length])
}
