use rustc_serialize::json::Json;

use std::io::BufReader;

use std::path::Path;
use std::fs::*;
use std::io::BufRead;
use std::io;
use std::io::Write;

#[derive(Debug, PartialEq, Eq)]
pub struct TelegramMessage {
    user_id:u64,
    message:String,
}

pub fn read_history(root_path:&Path, out:&Path) -> io::Result<()> {
    let mut file_out =  try!(OpenOptions::new().write(true).create(true).open(out));

    let path_result = try!(read_dir(root_path));
    let paths = path_result.map(|dir_entry| dir_entry.unwrap().path());

    for path in paths {
        println!("about to read path {:?}", path);
        println!("extension is {:?}", path.extension());
        
        let extension_ok = path.extension().map(|e| e == "jsonl").unwrap_or(false);
        
        if extension_ok {
            let file = try!(File::open(path));
            let reader = BufReader::new(file);
            let messages = reader.lines().filter_map(|l| message_for_line(&l.unwrap()) );
            for message in messages {
                let line = format!("{} {}\n", message.user_id, message.message);
                try!(file_out.write_all(line.as_bytes()));
            }
        }
    }


    try!(file_out.flush());

    Ok(())
}

pub fn message_for_line(str:&str) -> Option<TelegramMessage> {
    let json = Json::from_str(str).expect("json of some kind");
    let event_type = json.find("event").unwrap().as_string().expect("event type");


    if event_type == "message" {
        let from_user_id : Option<u64> = json.find_path(&["from","peer_id"]).and_then(|s| {
            s.as_u64() 
        });
        let message : Option<&str> = json.find("text").and_then(|x| x.as_string() );

        match (from_user_id, message) {
            (Some(um), Some(m)) => Some(TelegramMessage { user_id: um, message: super::persistence::clean_message(m) }),
            _ => None,
        }      
    } else {
        None
    }
}