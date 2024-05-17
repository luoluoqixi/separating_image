#![allow(dead_code)]

pub fn init_logger() {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{: <5}: {}: {}",
                record.level(),
                chrono::Local::now().format("%H:%M:%S"),
                message
            ))
        })
        .chain(
            fern::Dispatch::new()
                .level(log::LevelFilter::Warn)
                .level_for("separating_image", log::LevelFilter::Info)
                .chain(std::io::stdout()),
        )
        .apply()
        .unwrap();
}

#[cfg(target_os = "macos")]
pub fn pause() {
    use std::io::prelude::*;
    let mut stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    // We want the cursor to stay at the end of the line, so we print without a newline and flush manually.
    write!(stdout, "press enter key to continue...").unwrap();
    stdout.flush().unwrap();

    // Read a single byte and discard
    let _ = stdin.read(&mut [0u8]).unwrap();
}

pub fn system(command: &str) {
    let s = std::ffi::CString::new(command).unwrap();
    unsafe {
        libc::system((&s).as_ptr());
    }
}

#[cfg(target_os = "windows")]
pub fn pause() {
    system("pause")
}
