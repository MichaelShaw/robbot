use super::{HashSet};
use std::hash::Hash;
use std::ops::Range;
use std::fmt;

pub type Charset = HashSet<char>;
pub type WordSet = HashSet<String>;

pub fn char_range(from:char, to:char) -> Charset {
    let range : Range<u8> = (from as u8)..(to as u8) + 1;
    let char_vec : Vec<char> = range.map(|b| b as char ).collect();
    char_vec.into_iter().collect()
}

pub fn union<T : Hash + Eq + Clone>(lhs:&HashSet<T>, rhs:&HashSet<T>) -> HashSet<T> {
    lhs.union(rhs).cloned().collect()
}

pub fn word_set(words:Vec<&str>) -> WordSet {
    words.iter().map(|&st| String::from(st)).collect()
}

lazy_static! {
    static ref DIGITS : Charset = char_range('0','9');
    static ref PUNCTUATION : Charset = {
        vec!(',', '.', '(', ')', ':', ';', '/', '-', '?').into_iter().collect()
    };
    static ref LOWER_LETTERS : Charset = char_range('a','z');
    static ref UPPER_LETTERS : Charset = char_range('A', 'Z');
    static ref START_WORD : Charset = union(&union(&UPPER_LETTERS, &LOWER_LETTERS), &DIGITS);
    static ref CONTINUE_EXTRA : Charset =  {
        vec!('\'', '_').into_iter().collect() // apostrophe
    };
    static ref CONTINUE_WORD : Charset = union(&START_WORD, &CONTINUE_EXTRA);
    static ref LINK_WORDS : WordSet = word_set(vec!("http", "https", "www"));
}


pub fn as_string(c:char) -> String {
    let mut st = String::new();
    st.push(c);
    st
} 

enum ParseState {
    Word,
    Punctuation,
    Link,
    Whitespace,
}

#[derive(PartialEq, Eq, Clone, Hash)]
pub enum Token {
    Start,
    Word(String),
    Punctuation(String, bool),
    Link(String),
    End,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::Token::*;
        match self {
            &Start => write!(f, ".Start"),
            &Word(ref word) => write!(f, "{}", word),
            &Punctuation(ref punc, ref whitespace) => write!(f, "{}.sp?{}", punc, whitespace),
            &Link(ref link) => write!(f, "{}", link),
            &End => write!(f, ".End"),
        }
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::Token::*;
        match self {
            &Start => write!(f, ".Start"),
            &Word(ref word) => write!(f, "{}", word),
            &Punctuation(ref punc, ref whitespace) => write!(f, "{}.sp?{}", punc, whitespace),
            &Link(ref link) => write!(f, "{}", link),
            &End => write!(f, ".End"),
        }
    }
}

pub fn is_punctuation(token:&Token) -> bool {
    match token {
        &Token::Punctuation(_, _) => true,
        _ => false
    }
}

fn will_consume_char(st:&ParseState, c:char) -> bool {
    use self::ParseState::*;
    match st {
        &Word => CONTINUE_WORD.contains(&c),
        &Punctuation => PUNCTUATION.contains(&c),
        &Link => c != ' ',
        &Whitespace => c == ' ',
    }
}

fn create_token(st:&ParseState, string:&str, trailing_whitespace: bool) -> Option<Token> {
    use self::ParseState::*;
    match st {
        &Word => Some(Token::Word(String::from(string))),
        &Punctuation => Some(Token::Punctuation(String::from(string), trailing_whitespace)),
        &Link => Some(Token::Link(String::from(string))),
        &Whitespace => None,
    }
}

fn new_state_for_char(c:&char) -> ParseState {
    use self::ParseState::*;
    if START_WORD.contains(c) {
        Word
    } else if PUNCTUATION.contains(c) {
        Punctuation
    } else {
        Whitespace
    }
}

pub fn tokenize_line(line: &str) -> Vec<Token> {
    let mut parse_state = ParseState::Whitespace;
    let mut tokens : Vec<Token> = Vec::new();

    let mut token = String::new();

    tokens.push(Token::Start);

    for c in line.chars() {
        let consume = will_consume_char(&parse_state, c);
        if consume {
            token.push(c);
        } else {
            let is_whitespace = c == ' ';
            let evaluate_new_parser : bool = if !token.is_empty() {
                if LINK_WORDS.contains(&token) { // it's a link, keep the state
                    parse_state = ParseState::Link;
                    false
                } else if let Some(new_token) = create_token(&parse_state, &token, is_whitespace) {
                    tokens.push(new_token);
                    token.clear();
                    true
                } else {
                    token.clear(); // like whitespace doesnt emit a token
                    true
                }
            } else {
                true
            };
            if evaluate_new_parser {
                parse_state = new_state_for_char(&c);
            }
            token.push(c);
        }
    }

    if let Some(token) = create_token(&parse_state, &token, true) { // end of sentence is more whitespacey than not
        tokens.push(token);
    }

    tokens.push(Token::End);

    tokens
}