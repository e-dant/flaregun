static OUTF: std::sync::Mutex<Option<std::fs::File>> = std::sync::Mutex::new(None);

fn try_init_nonuniq(path: &std::path::PathBuf) -> Result<(), std::io::Error> {
    let mut file = OUTF.lock().unwrap();
    if file.is_none() {
        *file = Some(
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)?,
        );
    }
    Ok(())
}

pub fn init(path: &Option<std::path::PathBuf>) {
    static CALLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if CALLED.swap(true, std::sync::atomic::Ordering::SeqCst) {
        panic!("outf::init() called more than once");
    } else if let Some(path) = path {
        try_init_nonuniq(path).expect("Failed to open or create an output file");
    }
}

pub fn try_write_line(s: &str) -> Result<(), std::io::Error> {
    use std::io::Write;
    match OUTF.lock() {
        Ok(file) => match file.as_ref() {
            Some(mut file) => writeln!(file, "{s}"),
            None => Ok(println!("{s}")),
        },
        Err(_) => Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to lock output file",
        )),
    }
}

pub fn write_line(s: &str) {
    if let Err(e) = try_write_line(s) {
        log::error!("{e:?}");
        println!("{s}");
    }
}

const BYTES_BEFORE_FLUSH: usize = 1024 * 1024 * 100;

struct Buf {
    buf: [u8; BYTES_BEFORE_FLUSH],
    end: usize,
}

impl Buf {
    const fn new() -> Self {
        Self {
            buf: [0; BYTES_BEFORE_FLUSH],
            end: 0,
        }
    }

    fn write_line(&mut self, s: &str) {
        let len_with_newline = s.len() + 1;
        if len_with_newline > self.buf.len() {
            self.flush();
            write_line(s);
        } else {
            if self.end + len_with_newline > self.buf.len() {
                self.flush();
            }
            let end_with_newline = self.end + len_with_newline - 1;
            self.buf[self.end..end_with_newline].copy_from_slice(s.as_bytes());
            self.buf[end_with_newline] = b'\n';
            self.end += len_with_newline;
        }
    }

    fn flush(&mut self) {
        if self.end > 0 {
            match std::str::from_utf8(&self.buf[..self.end]) {
                Ok(s) => write_line(s),
                Err(e) => log::error!("{e:?}"),
            }
            self.end = 0;
        }
    }
}

static BUF: std::sync::Mutex<Buf> = std::sync::Mutex::new(Buf::new());

pub fn buf_write_line(s: &str) {
    if let Ok(mut buf) = BUF.lock() {
        buf.write_line(s);
    } else {
        log::error!("Lock failed, cannot write to output buffer");
    }
}

pub fn buf_flush() {
    if let Ok(mut buf) = BUF.lock() {
        buf.flush();
    } else {
        log::error!("Lock failed, cannot flush output buffer");
    }
}

// A `println!`-compatible macro that
// - Writes to the output file specified in a previous call to `init()`
// - Writes to stdout, if no output file was specified
// - Writes to stdout, if any errors occur while writing to the output file
// - Writes to stdout and `log::error!`s on any other errors
#[macro_export]
macro_rules! outfprintln {
    ($($arg:tt)*) => {
        $crate::outf::write_line(&format!($($arg)*))
    }
}

#[macro_export]
macro_rules! outfbufprintln {
    ($($arg:tt)*) => {
        $crate::outf::buf_write_line(&format!($($arg)*))
    }
}

pub use outfbufprintln;
pub use outfprintln;
