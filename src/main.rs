use native_tls::{TlsConnector, TlsStream, Certificate};
use std::fs::File;
use std::io::{Read, Write, stdin};
use std::net::TcpStream;

// Reads from the stream until it recieves '\r\n\r\n' sequence
fn read_until_2rn(stream: &mut TlsStream<TcpStream>, buf: &mut Vec<u8>) {
    let mut inner_buf = [0];
    let mut was_r = false;
    let mut rn_count = 0;
    while rn_count < 2 && match stream.read(&mut inner_buf) {
        Ok(size) => {
            if size == 1 {
                buf.push(inner_buf[0]);
                if inner_buf[0] == b'\r' {
                    was_r = true
                }
                else if inner_buf[0] == b'\n' && was_r {
                    rn_count += 1;
                    was_r = false;
                }
                else {
                    rn_count = 0;
                    was_r = false;
                }
            }
            true
        },
        Err(e) => {
            println!("Error while recieving message: {}", e);
            false
        }
    } {}
}

fn main() {
    let mut connector = TlsConnector::builder();

    let mut file = File::open("certificate.crt").unwrap();
    let mut certificate = vec![];
    file.read_to_end(&mut certificate).unwrap();
    let certificate = Certificate::from_pem(&certificate).unwrap();

    connector.add_root_certificate(certificate);
    let connector = connector.build().unwrap();

    let stream = TcpStream::connect("localhost:8080").unwrap();
    let mut stream = connector.connect("localhost", stream).unwrap();

    // Send message
    println!("Sending '{}'", "GET / HTTP/1.0\r\n\r\n");
    stream.write_all(b"GET / HTTP/1.0\r\n\r\n").unwrap();


    // Recieve response
    let mut res = vec![];
    read_until_2rn(&mut stream, &mut res);
    println!("Recieved '{}'", String::from_utf8_lossy(&res));

    // Send another message from user input
    let mut buf = String::new();
    stdin().read_line(&mut buf).unwrap();
    buf += "\r\n\r\n";
    println!("Sending '{}'", buf);
    stream.write_all(buf.as_bytes()).unwrap();

    // Close connection
    stream.shutdown().unwrap();
}