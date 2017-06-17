
use telegram_bot;
use telegram_bot::{Api, ParseMode, ListeningMethod, ListeningAction, User, Message, MessageType, Chat};

use command::*;
use model::*;
use persistence::*;
use history::*;
use unseeded_rng;
use tokenizer::*;
use generate::*;
use search::*;
use dice::*;

use glob::glob;

use rand::XorShiftRng;

use std::path::{PathBuf};

fn glob_vec(pattern: &str) -> Vec<PathBuf> {
    glob(pattern).unwrap().map(|r| r.unwrap()).collect()
}

pub struct Bot {
    model: Model,
    api: Api,
    listener: telegram_bot::Listener,
    persistence: Persistence,
    rand: XorShiftRng,
}

impl Bot {
    pub fn build(api_token: &str, chat_path: &str, history_path: &str) -> Result<Bot, telegram_bot::Error> {
        let chat_path = PathBuf::from(chat_path);
        let history_in_path = PathBuf::from(history_path);
        let history_out_path = chat_path.join("history.log");

        println!("chat {:?} history {:?} history_out {:?}", chat_path, history_path, history_out_path);

        let persistence = Persistence { root_path: chat_path.clone() };
        persistence.ensure_root().unwrap();
        if !file_exists_at(history_out_path.as_path()) {
            read_history(history_in_path.as_path(), history_out_path.as_path()).unwrap();
        }

        let glob_str = format!("{}/**/*.log", chat_path.to_str().unwrap());

        let paths = glob_vec(&glob_str);

        let model = create_models(paths);

        let api = Api::from_token(api_token)?;
        let listener = api.listener(ListeningMethod::LongPoll(None));

        let persistence = Persistence { root_path: chat_path.clone() };
        persistence.ensure_root().unwrap();

        Ok(Bot {
            model: model,
            api: api,
            listener: listener,
            persistence: persistence,
            rand: unseeded_rng()
        })
    }

    pub fn run(&mut self) -> Result<(), telegram_bot::Error> {
        use self::Response::*;

        let model = &self.model;
        let persistence_path = self.persistence.root_path.to_str().unwrap();
        let persistence = &self.persistence;
        let api = &self.api;
        let rng = &mut self.rand;

        self.listener.listen(|u| {
            match u.message {
                Some(Message { 
                    msg: MessageType::Text(t),
                    chat: Chat::Group {id:group_id, .. }, 
                    from,
                    .. }) => {
                    match handle(&from, group_id as u64, &t, &model, rng, persistence_path) {
                        Reply { msg, parse_mode } => {
                            match api.send_message(group_id, msg, parse_mode, None, None, None) {
                                Ok(_) => (),
                                Err(e) => println!("send message error -> {:?}", e),
                            }
                        },
                        Store { user_id, group_id, text } => {
                            persistence.store_chat_message(group_id, user_id, &text).expect("can persist chat message")
                        }
                    }
                },
                _ => (),        
            }
            Ok(ListeningAction::Continue)
        })
    }
}


pub enum Response {
    Reply { msg: String, parse_mode: Option<ParseMode> },
    Store { user_id: u64, group_id: u64, text: String}
}

pub fn handle(user:&User, group_id: u64, msg:&str, model:&Model, rand: &mut XorShiftRng, persistence_path: &str) -> Response {
    use self::Response::*;
    use self::ChatCommand::*;
    // use self::ChatModel::*;

    let words : Vec<String> = msg.trim().splitn(2, ' ').map(|t|t.to_lowercase()).collect();
    let command = words.first().and_then(|text| parse_command(text));

    let user_id = user.id.abs() as u64;

    if let Some(cmd) = command {
        

        match cmd {
            Roll => {
                if let Some(dice) = words.get(1).and_then(|text| parse_dice(&text)) {
                    let rolls : Vec<String> = dice.roll(rand).iter().map(|n| format!("{}", n) ).collect();
                    let roll_text = rolls.join(" ");
                    Reply { msg: format!("Rolled {}: {}", dice.to_string(), roll_text), parse_mode: None }
                } else {

                    Reply { msg: format!("Invalid dice"), parse_mode: None }
                }
            }
            Help => Reply { msg: String::from(HELP_MESSAGE), parse_mode: None },
            Generate(gen_mode) => {
                let (user_name, cm) = get_generative_model(model, &gen_mode, user_id, rand);

                let sentence_start = vec!(Token::Start);
                let message = generate(&model, rand, &sentence_start, &cm);
                Reply { msg: format!("{}: {}", user_name, message), parse_mode: None }
            },
            Finish(gen_mode) =>  {
                let (user_name, cm) = get_generative_model(model, &gen_mode, user_id, rand);

                let whatever = String::from("nf");
                let sentence_text : &str = words.get(1).unwrap_or(&whatever);
                let mut tokens = tokenize_line(sentence_text.to_lowercase().as_str());
                tokens.pop(); // remove the end
                let message = generate(&model, rand, &tokens, &cm);
                Reply { msg: format!("{}: {}", user_name, message), parse_mode: None }
            },
            Search(maybe_user) => {
                let user_ids = user_ids_for_chat_model(&maybe_user);
                let whatever = "".into();
                let sentence_text : &String = words.get(1).unwrap_or(&whatever);
                let terms = terms_for_search(sentence_text);

                let glob_str = format!("{}/**/*.log", persistence_path);
                let search_paths = glob_vec(&glob_str);
                
                let results = search(search_paths, &terms, &user_ids);
                let total = results.len();

                let mut message : String = format!("Searched for {:?} found {} results\n\n", terms, total);

                for result in results.iter().take(10) {
                    let user_name = username_for_id(result.user_id);
                    let some_shit = format!("{}: {}\n\n", user_name, pretty_search_result(&result.full_text, &terms));
                    message.push_str(&some_shit);
                } 

                Reply { msg: message, parse_mode: Some(ParseMode::Html) }
            },
        }
    } else {
        Store { user_id: user_id, group_id: group_id, text: String::from(msg) }
    }
}

const HELP_MESSAGE: &'static str = r#"
ctx: me|hydra|robe|mikel|michael

/roll 1d6
    Roll some dice bitch

/search
    Search all users

/search_{robe|mikel|michael}
    Search a specific users

/gen|/poke
    Sentence for random User

/gen_{ctx}
    Setence for contextual user

/finish <sentence start>
    Finish sentence for random user

/finish_{ctx} <sentence start>
    Finish for contextual user
"#;

