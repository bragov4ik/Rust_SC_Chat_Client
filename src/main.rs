use native_tls::{TlsConnector, TlsStream, Certificate};
use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;
use std::sync::{Mutex, Arc};
use std::str::FromStr;

mod chat_tui;
type Shared<T> = Arc<Mutex<T>>;

/// Writes all the buffer provided appending '\r\n\r\n' at the end
/// to indicate the end of message
fn send_to_stream(stream: &mut TlsStream<TcpStream>, buf: &[u8]) -> Result<(), std::io::Error> {
    let mut message: String = String::from_utf8_lossy(buf).to_string();
    message += "\r\n\r\n";
    stream.write_all(message.as_bytes())
}

/// Continuously reads from the stream (if any pending bytes present) or checks every `millis`
/// milliseconds until it receives '\r\n\r\n' sequence. 
/// Does not include the sequence in the result.
fn read_until_2rn(stream: & Shared<TlsStream<TcpStream>>, buf: &mut Vec<u8>, millis: u64) {
    let mut inner_buf = [0];
    let mut was_r = false;
    let mut rn_count = 0;
    while rn_count < 2 && match stream.lock().unwrap().read(&mut inner_buf) {
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
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
            thread::sleep(std::time::Duration::from_millis(millis));
            true
        }
        Err(e) => {
            chat_tui::draw_window(&vec!(format!("Error while receiving message: {}, kind: {:?}", e, e.kind())));
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
    let stream = connector.connect("localhost", stream).unwrap();
    let stream = Arc::new(Mutex::new(stream));

    // Print empty message
    chat_tui::open_window();
    chat_tui::draw_window(&vec!(""));

    // Value to stop iterating through a loop then we receive correct credentials
    let mut authentication_incorrect: bool = true;
    // Do not stop, until the credentials are correct
    while authentication_incorrect {
        chat_tui::draw_window(&vec!(
            "Log in format: login/password",
            "Registration format: login/password/username"));
        // Providing Credentials for Server
        let mut login_buf = String::new();
        chat_tui::read_input_line(&mut login_buf).unwrap();
        login_buf = login_buf.trim().to_string();
        send_to_stream(&mut stream.lock().unwrap(), login_buf.as_bytes()).unwrap();

        // Receive response from the server
        let mut res = vec![];
        read_until_2rn(&stream, &mut res, 100);
        let ind_cred_res = String::from_utf8_lossy(&res);

        // If the response says our credentials are correct, we stop iterating
        if ind_cred_res == "correct" {
            authentication_incorrect = false;
            chat_tui::draw_window(&vec!("Successfully logged in"));
        } else {
            // If the response says our credentials are incorrect, we continue iterating
            chat_tui::draw_window(&vec!(
                ind_cred_res.trim()));
        }
    }
    stream.lock().unwrap().get_ref().set_nonblocking(true).unwrap();
    let send_thread;
    let recv_thread;
    {
        let stream = stream.clone();
        send_thread = thread::spawn(move || {
            write_to_server(stream);
        });
    }
    {
        let stream = stream.clone();
        recv_thread = thread::spawn(move || {
            read_from_server(stream);
        });
    }
    send_thread.join().unwrap();

    // Close connection
    stream.lock().unwrap().shutdown().unwrap();

    chat_tui::close_window();
}

fn write_to_server(stream: Shared<TlsStream<TcpStream>>){
    loop{
        let mut msg_buf = String::new();
        chat_tui::read_input_line(&mut msg_buf).unwrap();
        msg_buf = String::from_str(msg_buf.trim()).unwrap();
        if msg_buf == "/exit" {
            break
        }
        send_to_stream(&mut stream.lock().unwrap(), msg_buf.as_bytes()).unwrap();
    }
}
fn read_from_server(stream: Shared<TlsStream<TcpStream>>) {
    let mut msg_vector: std::vec::Vec<String> = vec!();
    loop {
        let mut message = vec!();
        read_until_2rn(&stream, &mut message, 100);
        if String::from_utf8_lossy(&message).to_string() == "/exit" {
            break;
        }
        else if String::from_utf8_lossy(&message).to_string() == "" {
            continue;
        }
        msg_vector.insert(0, 
            String::from_utf8_lossy(&message).to_string()
        );
        chat_tui::draw_window(&msg_vector);
    }
}
