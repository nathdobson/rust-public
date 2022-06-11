extern crate libc;

use std::net::{TcpListener, TcpStream};
use std::os::raw::c_int;
use std::{io, mem};

pub fn set_reuse_port(listener: &TcpListener) {
    use std::os::unix::io::AsRawFd;
    unsafe {
        let optval: libc::c_int = 1;
        let ret = libc::setsockopt(
            listener.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_REUSEPORT,
            &optval as *const _ as *const libc::c_void,
            mem::size_of_val(&optval) as libc::socklen_t,
        );
        if ret != 0 {
            let err: io::Result<()> = Err(io::Error::last_os_error());
            err.expect("setsockopt failed");
        }
    }
}

pub fn set_reuse_addr(stream: &TcpListener) {
    use std::os::unix::io::AsRawFd;
    unsafe {
        let optval: libc::c_int = 1;
        let ret = libc::setsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_REUSEADDR,
            &optval as *const _ as *const libc::c_void,
            mem::size_of_val(&optval) as libc::socklen_t,
        );
        if ret != 0 {
            let err: io::Result<()> = Err(io::Error::last_os_error());
            err.expect("setsockopt failed");
        }
    }
}

pub fn set_linger(stream: &TcpStream) {
    use std::os::unix::io::AsRawFd;
    unsafe {
        #[repr(C)]
        struct linger {
            l_onoff: c_int,
            l_linger: c_int,
        }
        let optval = linger {
            l_onoff: 1,
            l_linger: 0,
        };
        let ret = libc::setsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_LINGER,
            &optval as *const _ as *const libc::c_void,
            mem::size_of_val(&optval) as libc::socklen_t,
        );
        if ret != 0 {
            let err: io::Result<()> = Err(io::Error::last_os_error());
            err.expect("setsockopt failed");
        }
    }
}
