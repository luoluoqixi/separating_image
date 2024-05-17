use lazy_static::lazy_static;
use std::path::PathBuf;

lazy_static! {
    pub static ref CURRENT_PATH: PathBuf = std::env::current_exe()
        .expect("failed to get app working directory")
        .parent()
        .expect("failed to get app working directory")
        .to_path_buf();
}
