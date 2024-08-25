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
            None => Ok(println!("{s}"))
        }
        Err(_) => Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to lock output file",
        )),
    }
}

pub fn write_line(s: &str) {
    if let Err(e) = try_write_line(s) {
        log::error!("{:?}", e);
        println!("{s}");
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

pub use outfprintln;
