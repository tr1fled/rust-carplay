use std::io::{self, prelude::*};
use std::net::TcpStream;

pub fn control_command(ip: String, host: String) -> io::Result<()> {
    // Connect to the server at 127.0.0.1:8080
    let mut stream = TcpStream::connect(ip)?;
    log::debug!("Connected to the server!");

    // Message to send to the server
    let message = format!("GET /ctrl-int/1/connect HTTP/1.1\nAirPlay-Receiver-Device-ID: 242606374384606\r\n\r\n");
    println!("{}", message);
    
    // Write the message as bytes to the stream
    stream.write_all(message.as_bytes())?;

    // Create a buffer to read the response
    let mut buffer = [0; 512]; // 512-byte buffer
    
    // Read data from the stream into the buffer
    let bytes_read = stream.read(&mut buffer)?;
    
    // Convert the received bytes to a string and print it
    let response = String::from_utf8_lossy(&buffer[..bytes_read]);
    log::debug!("Received: {}", response);

    Ok(())
}

pub fn send_command(ip: String, bplist: Vec<u8>) -> io::Result<()> {
    // Connect to the server at 127.0.0.1:8080
    // provisional
    let mut stream = TcpStream::connect(format!("{}:5000", ip))?;
    log::debug!("Connected to the server!");

    // Message to send to the server
    let message = format!("POST /command RTSP/1.0\nAirPlay-Receiver-Device-ID: 242606374384606\nContent-Type: application/x-apple-binary-plist\nContent-Length: {}\r\n\r\n", bplist.len());
    // Write the message as bytes to the stream
    stream.write_all(message.as_bytes())?;
    stream.write_all(&bplist.as_slice())?;

    // Create a buffer to read the response
    let mut buffer = [0; 512]; // 512-byte buffer
    
    // Read data from the stream into the buffer
    let bytes_read = stream.read(&mut buffer)?;
    
    // Convert the received bytes to a string and print it
    let response = String::from_utf8_lossy(&buffer[..bytes_read]);
    println!("Received: {}", response);

    Ok(())
}