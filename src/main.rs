use native_tls::{TlsConnector, TlsStream, Certificate};
use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;
use std::sync::{Mutex, Arc};

mod chat_tui;
type Shared<T> = Arc<Mutex<T>>;

fn send_to_stream(stream: &mut TlsStream<TcpStream>, buf: &[u8]) -> Result<(), std::io::Error> {
    let mut message: String = String::from_utf8_lossy(buf).to_string();
    message += "\r\n\r\n";
    stream.write_all(message.as_bytes())
}

/// Reads from the stream until it receives '\r\n\r\n' sequence
/// Does not include the sequence in the result
fn read_until_2rn(stream: &mut TlsStream<TcpStream>, buf: &mut Vec<u8>) {
    let mut inner_buf = [0];
    let mut was_r = false;
    let mut rn_count = 0;
    while rn_count < 2 && match stream.read(&mut inner_buf) {
        Ok(size) => {
            // If no bytes were read - just skip (maybe packet loss
            // or something)
            if size == 1 {
                // There are 5 'states' in the loop:
                // 0. was_r = false, rn_count = 0 - we haven't detected anything
                // 1. was_r = true, rn_count = 0 - we've detected '\r'
                // 2. was_r = false, rn_count = 1 - '\r\n'
                // 3. was_r = true, rn_count = 1 - '\r\n\r'
                // 4. was_r = false, rn_count = 2 - '\r\n\r\n', exit!
                if inner_buf[0] == b'\r' {
                    was_r = true
                } else if inner_buf[0] == b'\n' && was_r {
                    rn_count += 1;
                    was_r = false;
                } else {
                    // This wasn't an end of the message, add skipped
                    // bytes and continue fresh
                    if rn_count == 1 {
                        buf.push(b'\r');
                        buf.push(b'\n');
                    }
                    if was_r {
                        buf.push(b'\r');
                    }
                    buf.push(inner_buf[0]);
                    rn_count = 0;
                    was_r = false;
                }
            }
            true
        }
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

    // Print empty message
    chat_tui::open_window();
    chat_tui::draw_window(vec!(""));

    // Value to stop iterating through a loop then we receive correct credentials
    let mut authentication_incorrect: bool = true;
    // Do not stop, until the credentials are correct
    while authentication_incorrect {
        chat_tui::draw_window(vec!(
            "Log in format: login/password",
            "Registration format: login/password/username"));
        // Providing Credentials for Server
        let mut login_buf = String::new();
        chat_tui::read_input_line(&mut login_buf).unwrap();
        login_buf = login_buf.trim().to_string();
        send_to_stream(&mut stream, login_buf.as_bytes()).unwrap();

        // Receive response from the server
        let mut res = vec![];
        read_until_2rn(&mut stream, &mut res);
        let ind_cred_res = String::from_utf8_lossy(&res);

        // If the response says our credentials are correct, we stop iterating
        if ind_cred_res == "correct" {
            authentication_incorrect = false
        } else {
            // If the response says our credentials are incorrect, we continue iterating
            chat_tui::draw_window(vec!(
                ind_cred_res.trim()));
        }
    }
    let stream = Arc::new(Mutex::new(stream));
{
    let stream = stream.clone();
    thread::spawn(move || {
        write_to_server(stream);
    });
}
{
    let stream = stream.clone();
    thread::spawn(move || {
        read_from_server(stream);
    });
}
    chat_tui::close_window();



    // Close connection
    stream.lock().unwrap().shutdown().unwrap();
}

fn write_to_server(stream: Shared<TlsStream<TcpStream>>){
    loop{
        let mut msg_buf = String::new();
        chat_tui::read_input_line(&mut msg_buf).unwrap();
        send_to_stream(&mut stream.lock().unwrap(), msg_buf.as_bytes()).unwrap();
    }
}
fn read_from_server(stream: Shared<TlsStream<TcpStream>>) {
    loop {
        let mut message = vec![];
        read_until_2rn(&mut stream.lock().unwrap(), &mut message);

    }
}
