use std::io::Write;
use std::net::{Shutdown, TcpListener};
use std::path::Path;
use std::{io, thread};

pub fn start_web_server(addr: &str, filename: &Path) -> io::Result<()> {
    let listener = TcpListener::bind(addr)?;
    let content = std::fs::read_to_string(filename)?;
    let mut packet = Vec::new();

    write!(&mut packet, "HTTP/1.0 200 OK\r\n")?;
    write!(&mut packet, "Content-Length: {}\r\n", content.len())?;
    write!(&mut packet, "Content-Type: application/octet-stream\r\n")?;
    write!(
        &mut packet,
        "Content-Disposition: attachment; filename=\"{}\"\r\n",
        filename.file_name().unwrap().to_str().unwrap()
    )?;
    write!(&mut packet, "\r\n")?;
    write!(&mut packet, "{}", content)?;
    thread::spawn(move || loop {
        let (mut socket, _) = listener.accept().unwrap();
        if let Err(e) = || -> io::Result<()> {
            socket.set_nonblocking(true)?;
            socket.write(&packet)?;
            socket.shutdown(Shutdown::Write)?;
            Ok(())
        }() {
            eprintln!("HTTP error: {:?}", e);
        }
    });
    Ok(())
}
