use std::net::TcpListener;
use std::{mem, io};
extern crate libc;

pub fn fix_listener(listener: &TcpListener) {
    use std::os::unix::io::AsRawFd;
    unsafe {
        let optval: libc::c_int = 1;
        let ret = libc::setsockopt(listener.as_raw_fd(),
                                   libc::SOL_SOCKET,
                                   libc::SO_REUSEPORT,
                                   &optval as *const _ as *const libc::c_void,
                                   mem::size_of_val(&optval) as libc::socklen_t);
        if ret != 0 {
            let err: io::Result<()> = Err(io::Error::last_os_error());
            err.expect("setsockopt failed");
        }
    }
}