use chrono::*;

use std::path::{PathBuf, Path};
use std::fs::*;
use std::io;
use std::io::{Error, ErrorKind};
use std::io::Write;

pub fn file_exists_at(path:&Path) -> bool {
    path.is_file() && path.exists()
}

pub fn ensure_directory(path:&Path) -> io::Result<()> {
    if path.is_dir() {
        Ok(())
    } else if path.is_file() {
        Err(Error::new(ErrorKind::AlreadyExists, format!("Oh no, {:?} is a file, not a directory", path)))
    } else { // doesn't exist
        try!(create_dir_all(path));
        Ok(())
    }  
}

pub fn clean_message(message:&str) -> String {
    message.replace("\n"," ")
}

pub struct Persistence {
    pub root_path : PathBuf,
}

impl Persistence {
    pub fn ensure_root(&self) -> io::Result<()> {
        let path = self.root_path.as_path();
        ensure_directory(path)
    }

    pub fn store_chat_message(&self, group: u64, user: u64, message:&str) -> io::Result<()> {
        let mut group_path = self.root_path.clone();
        group_path.push(group.to_string());

        try!(ensure_directory(group_path.as_path()));

        let dt = Local::now();
        let date_string = dt.format("%Y-%m-%d").to_string();

        group_path.push(format!("{}.log", date_string));

        let mut file = try!(OpenOptions::new().create(true).append(true).open(group_path.as_path()));
        
        let cleaned_message = clean_message(message);
        let line = format!("{} {}\n", user, cleaned_message);
        try!(file.write_all(line.as_bytes()));
        
        try!(file.flush());
        
        Ok(())
    }
}
