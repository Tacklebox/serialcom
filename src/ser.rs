extern crate serial;

use std::env;
use std::io;
use std::time::Duration;
use std::thread::sleep;

use std::io::prelude::*;
use serial::prelude::*;

fn main() {
    for arg in env::args_os().skip(1) {
        let mut port = serial::open(&arg).unwrap();
        sleep(Duration::from_millis(1000));
        interact(&mut port).unwrap();
    }
}

fn interact<T: SerialPort>(port: &mut T) -> io::Result<()> {
    try!(port.reconfigure(&|settings| {
        settings.set_baud_rate(serial::Baud9600);
        settings.set_char_size(serial::Bits8);
        settings.set_stop_bits(serial::Stop1);
        settings.set_flow_control(serial::FlowNone);
        Ok(())
    }));

    try!(port.set_timeout(Duration::from_millis(2000)));

    let mut buf: Vec<u8> = vec!('7' as u8);
    try!(port.write(&buf));
    try!(port.read(&mut buf[..]));
    println!("Read {}", std::str::from_utf8(&buf).unwrap());

    Ok(())
}
