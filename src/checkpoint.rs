extern crate serial;
extern crate termion;
extern crate tui;

use serial::prelude::*;

use std::env;
use std::io;
use std::io::prelude::*;
use std::thread;
use std::time;
use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver};

use termion::event;
use termion::input::TermRead;

use tui::Terminal;
use tui::backend::MouseBackend;
use tui::layout::{Direction, Group, Rect, Size};
use tui::widgets::{Block, Borders, Paragraph, Widget};

enum Event {
    Input(event::Key),
    Msg(String),
    ClosePort
}

const SETTINGS: serial::PortSettings = serial::PortSettings {
    baud_rate:    serial::Baud9600,
    char_size:    serial::Bits8,
    parity:       serial::ParityNone,
    stop_bits:    serial::Stop1,
    flow_control: serial::FlowNone,
};

fn main() {
    let mut port = serial::open("/dev/ttyACM0").unwrap();
    // Prepare terminal
    let mut terminal = Terminal::new(MouseBackend::new().unwrap()).unwrap();
    terminal.clear().unwrap();
    terminal.hide_cursor().unwrap();

    // Create Synchronization for user/serial port events
    let (thread_tx, main_rx): (Sender<Event>, Receiver<Event>) = mpsc::channel();
    let (main_tx, thread_rx): (Sender<Event>, Receiver<Event>) = mpsc::channel();

    // User presses a key
    let user_input_event = thread_tx.clone();
    // Serial port receives data
    let recv_data_event = thread_tx.clone();

    thread::spawn(move || {
        let stdin = io::stdin();
        for c in stdin.keys() {
            let evt = c.unwrap();
            user_input_event.send(Event::Input(evt)).unwrap();
            if evt == event::Key::Esc {
                break;
            }
        }
    });

    thread::spawn(move || {
        port.configure(&SETTINGS).expect("Unable to configure serial port");
        port.set_timeout(time::Duration::from_millis(50)).expect("Unable to configure serial port");
        let mut serial_buffer = Vec::new();
        loop {
            match thread_rx.recv_timeout(time::Duration::from_millis(50)) {
                Ok(Event::Msg(mess)) => {
                    serial_buffer = mess.clone().into_bytes();
                    serial_buffer.shrink_to_fit();
                    port.write(&serial_buffer).expect("Unable to write to port");
                },
                Ok(Event::ClosePort) => (),
                Err(_) => (),
                Ok(Event::Input(_)) => ()
            }
            serial_buffer.reserve(200);
            match port.read(&mut serial_buffer[..]) {
                Ok(_) => {
                    serial_buffer.shrink_to_fit();
                    recv_data_event.send(Event::Msg(String::from_utf8(serial_buffer.clone()).unwrap())).expect("Channel Broken");
                },
                Err(io::Error {io::ErrorKind(e), ..}) => {
                    panic!("{:?}",e);
                }
            }
        }
    });


    thread::sleep(time::Duration::from_millis(1000));

    let mut user_input = "".to_string();
    let mut serial_output = "".to_string();
    let mut cursor_position = 0;
    let mut term_size = terminal.size().unwrap();

    draw(&mut terminal, &term_size, "{mod=invert  }", "");
    loop {
        let evt = main_rx.recv().unwrap();
        match evt {
            Event::Input(key) => match key {
                event::Key::Esc => break,
                event::Key::Left => {
                    if cursor_position > 0 {
                        cursor_position -= 1;
                    }
                }
                event::Key::Right => {
                    if cursor_position < user_input.len() {
                        cursor_position += 1;
                    }
                }
                event::Key::Backspace => {
                    if cursor_position > 0 {
                        user_input.remove(cursor_position - 1);
                        cursor_position -= 1;
                    }
                }
                event::Key::Delete => {
                    if cursor_position < user_input.len() {
                        user_input.remove(cursor_position);
                    }
                }
                event::Key::Char(the_char) => match the_char {
                    '\n' => {
                        main_tx.send(Event::Msg(user_input)).unwrap();
                        user_input = "".to_string();
                        cursor_position = 0;
                    }
                    '\t' => (),
                    _ => {
                        user_input.insert(cursor_position, the_char);
                        cursor_position += 1;
                    }
                },
                _ => break,
            },
            Event::Msg(mess) => {
                serial_output = serial_output+&mess;
            },
            Event::ClosePort =>()
        }
        let size = terminal.size().unwrap();
        if term_size != size {
            terminal.resize(size).unwrap();
            term_size = size;
        }
        let mut input_with_cursor = user_input.clone();
        if cursor_position == input_with_cursor.len() {
            input_with_cursor.push_str("{mod=invert  }");
        } else {
            input_with_cursor.insert(cursor_position + 1, '}');
            for n in "{mod=invert ".chars().rev() {
                input_with_cursor.insert(cursor_position, n);
            }
        }
        draw(&mut terminal, &term_size, &input_with_cursor, &serial_output);
    }
    terminal.clear().unwrap();
    terminal.show_cursor().unwrap();
}

fn draw(t: &mut Terminal<MouseBackend>, size: &Rect, user_input: &str, serial_output: &str) {
    Block::default().borders(Borders::ALL).render(t, size);
    Group::default()
        .direction(Direction::Vertical)
        .margin(1)
        .sizes(&[Size::Fixed(3), Size::Min(0)])
        .render(t, size, |t, chunks| {
            Paragraph::default()
                .block(Block::default().borders(Borders::ALL))
                .text(user_input)
                .render(t, &chunks[0]);
            Paragraph::default()
                .block(Block::default().borders(Borders::ALL).title("Serial"))
                .text(serial_output)
                .render(t, &chunks[1]);
        });

    t.draw().unwrap();
}
