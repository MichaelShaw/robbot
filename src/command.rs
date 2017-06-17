
extern crate rand;

use rand::Rng;
use super::{HashMap, HashSet};
use super::model::*;
use super::generate::choose_user;

macro_rules! hashmap {
    ($( $key: expr => $val: expr ),*) => {{
         let mut map = HashMap::default();
         $( map.insert($key, $val); )*
         map
    }}
}

lazy_static! {
    static ref ID_TO_CASUAL_NAME : HashMap<UserId, String> = hashmap![101710896 => String::from("michael"), 99688863 => String::from("robe"), 91597707 => String::from("michael")];
    static ref USER_LOOKUP_CASUAL : HashMap<String, UserId> = hashmap![String::from("michael") => 101710896, String::from("robe") => 99688863, String::from("mikel") => 91597707];
    static ref USER_LOOKUP : HashMap<UserId, String> = hashmap![101710896 => String::from("Michael Shaw"), 99688863 => String::from("Robe"), 91597707 => String::from("Michael Grogan")];
}

pub fn casual_usernames() -> Vec<String> {
    USER_LOOKUP_CASUAL.keys().cloned().collect()
}

pub fn all_user_ids() -> Vec<UserId> {
    ID_TO_CASUAL_NAME.keys().cloned().collect()
}

pub fn casual_name_for_id(id:UserId) -> String {
    ID_TO_CASUAL_NAME.get(&id).unwrap().clone()
}

pub fn user_id_for_casual(name:&str) -> Option<UserId> {
    USER_LOOKUP_CASUAL.get(name).cloned()
}

pub fn username_for_id(id:UserId) -> String {
    USER_LOOKUP.get(&id).unwrap().clone()
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub enum ChatCommand {
    Help,
    Search(Option<String>),
    Generate(ChatModel),
    Finish(ChatModel),
    Roll,
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum ChatModel {
    Me,
    Random,
    All,
    User(String),
}


pub fn parse_user(model: &str) -> Option<String> {
    if USER_LOOKUP_CASUAL.contains_key(model) {
        Some(String::from(model))
    } else {
        None
    }
}

pub fn parse_model(model: &str) -> Option<ChatModel> {
    if model == "me" {
        Some(ChatModel::Me)
    } else if model == "hydra" {
        Some(ChatModel::All)
    } else if model == "random" {
        Some(ChatModel::Random)
    } else if USER_LOOKUP_CASUAL.contains_key(model) {
        Some(ChatModel::User(String::from(model)))
    } else {
        None
    }
}

pub fn user_ids_for_chat_model(username: &Option<String>) -> HashSet<UserId> {
    let mut user_ids : HashSet<UserId> = HashSet::default();
    
    if let &Some(ref name) = username {
        let user_id = user_id_for_casual(name).unwrap();
        user_ids.insert(user_id);
    } else {
        for user_id in all_user_ids() {
            user_ids.insert(user_id);
        } 
    }

    user_ids
}


pub fn parse_command(command: &str) -> Option<ChatCommand> {
    let parts : Vec<&str> = command.split("_").collect();

    match (parts.first(), parts.get(1)) {
        (Some(&"/roll"), _) => {
           Some(ChatCommand::Roll)
        }
        (Some(&"/help"), _) => {
            Some(ChatCommand::Help)
        }
        (Some(&"/search"), maybe_model) => {
            let maybe_user = maybe_model.and_then(|m| parse_user(m));
            Some(ChatCommand::Search(maybe_user))
        }
        (Some(&"/hydra"), _) => Some(ChatCommand::Generate(ChatModel::All)),
        (Some(&"/poke"), _) => Some(ChatCommand::Generate(ChatModel::Random)),
        (Some(&"/finish"), maybe_model) => {
            let some_shit = maybe_model.and_then(|m| parse_model(m)).unwrap_or(ChatModel::Random);
            Some(ChatCommand::Finish(some_shit))
        },
        (Some(&"/gen"), maybe_model) => {
            let some_shit = maybe_model.and_then(|m| parse_model(m)).unwrap_or(ChatModel::Random);
            Some(ChatCommand::Generate(some_shit))
        },
        _ => None,
    }
}

pub fn get_generative_model<'a, R : Rng>(m: &'a Model, chat_model:&ChatModel, user_id: UserId, rng: &mut R) -> (String, &'a UserGenerativeModel) {
    match chat_model {
        &ChatModel::Me => (username_for_id(user_id).clone(), &m.users[&user_id]),
        &ChatModel::All => ("hydra".into(), &m.shared),
        &ChatModel::User(ref name) => {
            let user_id = user_id_for_casual(name).unwrap();
            (username_for_id(user_id), &m.users[&user_id])
        },
        &ChatModel::Random => {
            let user_id = choose_user(&m, rng);
            let name = username_for_id(user_id);
            (String::from(name), &m.users[&user_id])
        },
    }
}