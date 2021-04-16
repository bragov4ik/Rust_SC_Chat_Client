//! # Chat-tui
//! It is a module that provides very simple terminal user interface
//! designed for chat application.
//! # Getting started
//! Before using the module, it is required to execute function 
//! `open_window()`, which opens terminal's alternative screen
//! and does other preparations.
//! 
//! After finishing working with the module, `close_window()`
//! will close the alternative screen, leaving the user at the
//! same position in main terminal screen as before `open_window()`.
//! 
//! Main function of the module - `draw_window` which outputs
//! all given messages that fit onto the screen.
//! 
//! # Known problems
//! * Changing size of the terminal breaks the interface until
//! next `draw_window()`.
//! 

use std::io::{stdout, stdin, Write};
use std::fmt;
use std::str::FromStr;

use crossterm::cursor::{
    MoveToPreviousLine, 
    MoveToNextLine, 
    MoveToColumn, 
    SavePosition, 
    RestorePosition
};
use crossterm::terminal::{Clear, ClearType};
use crossterm::execute;
use crossterm::queue;
use crossterm::style::Print;

struct Timestamp {
    time: String
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}]", self.time)
    }
}

impl Clone for Timestamp {
    fn clone(&self) -> Timestamp {
        Timestamp {
            time: self.time.clone(),
        }
    }
}

pub struct Message {
    timestamp: Timestamp,
    author: String,
    contents: String,
}

impl Message {
    pub fn from_strings(time: String, author: String, contents: String) -> Message{
        Message {
            timestamp: Timestamp{time},
            author,
            contents,
        }
    }
    fn from_raw(timestamp: Timestamp, author: String, contents: String) -> Message{
        Message {
            timestamp,
            author,
            contents,
        }
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}:{}", self.timestamp, self.author, self.contents)
    }
}

impl Clone for Message {
    fn clone(&self) -> Message {
        Message::from_raw(self.timestamp.clone(), self.author.clone(), self.contents.clone())
    }
}

/// Builds vector of strings with messages in reverse order
/// of arrival (newest - last) and in sequental order within
/// a message. Those lines do not exceed `columns` by using
/// line break.
/// 
/// Requires messages sorted in order of newest first
/// 
/// # Arguments
/// 
/// * `messages: Vec<Message>` - list of messages sorted in newest first order
/// * `max_rows` - number of lines after exceeding which it stops adding messages.
/// Does not guarantee that result (in lines) will be shorter than this value, it
/// is used for optimization purposes to not add unnecessary lines.
/// * `columns` - maximum result line length
/// 
/// # Examples
/// This program constructs lines of messages for frame
/// with size 8x5 and prints them.
/// ```
/// mod chat_tui;
/// 
/// use std::str::FromStr;
/// 
/// 
/// fn main() {
///     let test_vec = vec!(
///         chat_tui::Message::from_raw(
///             String::from_str("123").unwrap(), 
///             String::from_str("aboba").unwrap(), 
///             String::from_str("ABOBA").unwrap(),
///         ),
///         chat_tui::Message::from_raw(
///             String::from_str("122").unwrap(), 
///             String::from_str("cock").unwrap(), 
///             String::from_str("cam").unwrap(),
///         ),
///         chat_tui::Message::from_raw(
///             String::from_str("32").unwrap(), 
///             String::from_str("cockerel").unwrap(), 
///             String::from_str("beef").unwrap(),
///         ),
///     );
///     let result = chat_tui::build_messages_string_arr(test_vec, 8, 5);
///     for res in result {
///         println!("{}", res);
///     }
/// }
/// ```
pub fn build_messages_string_list<T: fmt::Display>(messages: Vec<T>, max_rows: usize, columns: usize) -> Vec<String> {
    let mut result = vec!();
    for msg in messages {
        // Stop on exceeding terminal frame
        if result.len() >= max_rows {
            break;
        }

        let msg_string: String = format!("{}", msg);
        let mut msg_vector = vec!();

        // TODO proper length operation to work with multibyte
        // characters properly

        for i in (0..msg_string.len()).step_by(columns) {
            let mut end_index = i+columns;
            if end_index >= msg_string.len() {
                end_index = msg_string.len();
            }
            // This will likely to panic in case of multi-byte characters
            msg_vector.push(String::from_str(&msg_string[i..end_index]).unwrap());
        }
        msg_vector.append(&mut result);
        result = msg_vector;
    }
    result
}

/// Writes messages within specified frame. Assumes that cursor is located at the
/// lower left corner of the frame. Leaves cursor at the same column of upper line.
fn add_messages<T: fmt::Display>(messages: Vec<T>, max_rows: usize, columns: usize) {
    let messages_list = build_messages_string_list(messages, max_rows, columns);
    let messages_list_iter = messages_list.into_iter().rev();
    let start_column = crossterm::cursor::position().unwrap().0;
    let mut cur_line = 0;
    for msg in messages_list_iter {
        if cur_line >= max_rows {
            break;
        }
        queue!(
            stdout(),
            Print(msg),
            MoveToColumn(start_column),
            MoveToPreviousLine(1)
        ).unwrap();
        cur_line += 1;
    }
}

/// Adds commands to move one line up and go to the first column
/// in the queue.
fn add_up_home() {
    queue!(
        stdout(),
        MoveToPreviousLine(1),
        MoveToColumn(0),
    ).unwrap();
}

/// Adds command to write separating line for the whole row
/// in the queue. 
fn add_delimiter_line(columns: usize) {
    queue!(
        stdout(), 
        Print("-".repeat(columns)),
    ).unwrap();
}

/// Adds command to draw line for contents in the queue.
fn add_empty_line(columns: u16) {
    queue!(
        stdout(),
        MoveToColumn(columns),
    ).unwrap(); 
}

/// Draws window with provided messages
/// 
/// # Arguments
/// 
/// * `messages` - vector with entries of any type that implement [`Display`](https://doc.rust-lang.org/std/fmt/trait.Display.html) trait ordered by newest.
/// Newest message will be printed at the bottom of the screen.
/// 
pub fn draw_window<T: fmt::Display>(messages: Vec<T>) {

    let (columns_u16, rows_u16) = crossterm::terminal::size().unwrap();
    queue!(
        stdout(), 
        crossterm::terminal::SetSize(columns_u16, rows_u16)
    ).unwrap();
    let (columns, rows): (usize, usize) = (columns_u16.into(), rows_u16.into());
    queue!(
        stdout(), 
        SavePosition,
    ).unwrap();
    // Move to the first column of line after input field
    move_cursor_to_input_field();
    queue!(
        stdout(),
        MoveToNextLine(1),
        MoveToColumn(0),
    ).unwrap();
    add_delimiter_line(columns);
    add_up_home();
    add_empty_line(columns_u16);
    add_up_home();
    add_delimiter_line(columns);
    add_up_home();
    
    // Erase old messages
    queue!(
        stdout(),
        Clear(ClearType::CurrentLine),
        Clear(ClearType::FromCursorUp),
    ).unwrap();

    // Write messages
    add_messages(messages, rows-3, columns);
    add_up_home();

    add_delimiter_line(columns);
    add_up_home();

    queue!(
        stdout(),
        RestorePosition
    ).unwrap();
    stdout().flush().unwrap();
}

fn move_cursor_to_input_field() {
    let rows_u16 = crossterm::terminal::size().unwrap().1;
    execute!(
        stdout(), 
        crossterm::cursor::MoveTo(0, rows_u16-2)
    ).unwrap();
}

pub fn clear_input_field() {
    execute!(
        stdout(),
        SavePosition,
    ).unwrap();
    move_cursor_to_input_field();
    execute!(
        stdout(),
        Clear(ClearType::CurrentLine),
        RestorePosition,
    ).unwrap();
}

pub fn open_window() {
    execute!(
        stdout(), 
        crossterm::terminal::EnterAlternateScreen
    ).unwrap();

    move_cursor_to_input_field();
}

pub fn close_window() {
    execute!(
        stdout(), 
        crossterm::terminal::LeaveAlternateScreen
    ).unwrap();
}

pub fn read_input_line(buf: &mut String) -> std::io::Result<usize> {
    let result = stdin().read_line(buf);
    clear_input_field();
    move_cursor_to_input_field();
    result
}