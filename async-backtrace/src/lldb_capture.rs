use tokio::process::{Command, ChildStdout, ChildStdin, Child};
use std::process::{Stdio};
use std::io;
use std::convert::{TryInto, TryFrom};
use tokio::io::{AsyncReadExt, BufReader, AsyncBufReadExt, AsyncSeekExt};
use std::io::{Seek, SeekFrom, Read};
use tokio::task::spawn_blocking;
use tokio::fs::File;
use std::fmt::Write;
use itertools::Itertools;
use std::ffi::c_void;
use crate::remangle::resolve_remangle;

async fn tempfile() -> io::Result<File> {
    spawn_blocking(|| Ok(File::from_std(tempfile::tempfile()?))).await?
}

pub async fn capture() -> io::Result<String> {
    let mut stdout = tempfile().await?;
    let mut stderr = tempfile().await?;

    let mut lldb = Command::new("lldb")
        .arg("-p").arg(format!("{}", std::process::id()))
        .arg("-o").arg("settings set thread-stop-format 'THREAD;{${thread.index}};{${thread.name}};{${thread.queue}};\\n'")
        .arg("-o").arg("settings set frame-format 'FRAME;{${frame.index}};{${frame.pc}};\\n'")
        .arg("-o").arg("thread backtrace all")
        .arg("-o").arg("exit")
        .stdin(Stdio::null())
        .stdout(stdout.try_clone().await?.into_std().await)
        .stderr(stderr.try_clone().await?.into_std().await)
        .spawn()?;

    let mut output = String::new();
    let status = lldb.wait().await?;
    if !status.success() {
        writeln!(output, "lldb exit status {}", status).unwrap();
    }

    stderr.seek(SeekFrom::Start(0)).await?;
    stderr.read_to_string(&mut output).await?;

    stdout.seek(SeekFrom::Start(0)).await?;
    let mut stdout = BufReader::new(stdout);
    let mut line = String::new();
    loop {
        line.clear();
        if stdout.read_line(&mut line).await? == 0 {
            break;
        }
        if translate_line(&line, &mut output).is_none() {
            write!(output, "lldb: {}", line).unwrap();
        }
    }
    Ok(output)
}

fn translate_line(line: &str, output: &mut String) -> Option<()> {
    let line = line.trim_matches(|x: char| x.is_whitespace() || x == '*');
    let mut iter = line.split(";");
    let typ = iter.next()?;
    if typ == "THREAD" {
        let (index, name, queue) = iter.next_tuple()?;
        writeln!(output).unwrap();
        writeln!(output, "thread index={} name={} queue={}", index, name, queue).unwrap();
    } else if typ == "FRAME" {
        let (index, pc) = iter.next_tuple()?;
        let pc = usize::from_str_radix(pc.trim_start_matches("0x"), 16).ok()?;
        let symbols = resolve_remangle(pc as *mut c_void);
        for symbol in symbols {
            writeln!(output, "{}", symbol).unwrap();
        }
    } else {
        return None;
    }
    Some(())
}

#[tokio::test]
async fn test_lldb() {
    println!("{}", capture().await.unwrap());
}