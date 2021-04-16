use native_tls::{TlsConnector, TlsStream, Certificate};
use std::fs::File;
use std::io::{Read, Write, stdin};
use std::net::TcpStream;

// Reads from the stream until it receives '\r\n\r\n' sequence
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
            println!("Error while receiving message: {}", e);
            false
        }
    } {}
}

fn main() {
    // Tool to establish TLS connections
    let mut connector = TlsConnector::builder();

    // Unfolding and usage of certificate for a user
    let mut file = File::open("certificate.crt").unwrap();
    let mut certificate = vec![];
    file.read_to_end(&mut certificate).unwrap();
    let certificate = Certificate::from_pem(&certificate).unwrap();
    connector.add_root_certificate(certificate);

    let connector = connector.build().unwrap();
    let stream = TcpStream::connect("localhost:8080").unwrap();
    let mut stream = connector.connect("localhost", stream).unwrap();

    // Value to stop iterating through a loop then we receive correct credentials
    let mut authentication_incorrect: bool = true;
    // Do not stop, until the credentials are correct
    while authentication_incorrect {
        println!("Log in format: login/password");
        println!("Registration format: login/password/username");
        // Providing Credentials for Server
        let mut login_buf = String::new();
        stdin().read_line(&mut login_buf).unwrap();
        login_buf += "\r\n\r\n";
        stream.write_all(login_buf.as_bytes()).unwrap();

        // Receive response from the server
        let mut res = vec![];
        read_until_2rn(&mut stream, &mut res);
        let ind_cred_res = String::from_utf8_lossy(&res);

        // If the response says our credentials are correct, we stop iterating
        if ind_cred_res == "correct\r\n\r\n" {
            authentication_incorrect = false
        } else {
            // If the response says our credentials are incorrect, we continue iterating
            println!("{}",ind_cred_res.trim_end_matches("\r\n\r\n"));
        }
    }

    // Close connection
    stream.shutdown().unwrap();
}