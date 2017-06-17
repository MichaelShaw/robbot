extern crate rand;

use super::model::*;
use rand::Rng;
use super::tokenizer::{Token};
use super::HashMap;
use std::hash::Hash;

pub type BigramDebug = Vec<(Token, OccurenceCount, Vec<(Token, f64)>)>;
pub type TrigramDebug = Vec<((Token, Token), OccurenceCount, Vec<(Token, f64)>)>;
 

pub struct GenerationDebugInfo {
    pub bigrams: BigramDebug,
    pub trigrams: TrigramDebug,
}

impl Default for GenerationDebugInfo {
    fn default() -> GenerationDebugInfo { 
        GenerationDebugInfo {
            bigrams: Vec::new(),
            trigrams: Vec::new()
        }
    }
}

pub fn generate<R : Rng>(model:&Model, rng: &mut R, sentence_start:&Vec<Token>, user_model:&UserGenerativeModel) -> String { // -> (UserId, String, GenerationDebugInfo)
    let to_idx = |t:&Token| -> TokenIdx {
        *model.token_to_idx.get(t).unwrap()
    };
    let to_token = |ti:TokenIdx| -> Token {
        model.tokens[ti].clone()
    };

    let mut line : Vec<Token> = Vec::new();
    for token in sentence_start {
        line.push(token.clone());
    }

    let last_resort = GeneratedToken {
        token_idx: to_idx(&Token::End),
        chosen_occurrences: 0,
        table_occurrences: 0,
        popular: Vec::new(),
    };

    println!("starting line -> {:?}", line);
    
    while line.last() != Some(&Token::End) && line.len() < 30 {
        let mut selections : Vec<(Option<GeneratedToken>, f64)> = Vec::new();

        let trigram_selection = user_model.own_trigrams.generate(&line, line.len(), &model.token_to_idx, rng);
        let bigram_selection = user_model.own_bigrams.generate(&line, line.len(), &model.token_to_idx, rng);
        
        selections.push((trigram_selection.clone(),0.74));
      
        let primary_selection : Option<GeneratedToken> = selections.into_iter().filter(|&(ref opt, probability)| {
            match opt {
                &Some(ref generated_token) => {
                    let token = to_token(generated_token.token_idx);
                    if generated_token.table_occurrences <= 1 {
                        println!("=== rejected table ==== token \"{:?}\" with p {:2} ", token, probability);
                        false
                    }  else {
                        let roll = rng.next_f64();
                        let take = roll <= probability;
                        println!("=== rolling for === token \"{:?}\" with p {:2} roll {:2} take? {:}", token, probability, roll, take);
                        take
                    } 
                }
                &None => false,
            }
        }).next().and_then(|(x,_)| x); // .clone()

        let generated_token = primary_selection.or(bigram_selection.clone()).unwrap_or_else(|| last_resort.clone()); // last resort is naive bigram
        let token = to_token(generated_token.token_idx);

        println!("{:20} tri {:120} bi {:120}", 
            format_token(&token),
            format_selection(&trigram_selection, &model.tokens), 
            format_selection(&bigram_selection, &model.tokens),
        );

        line.push(token);
    }    

    generate_sentence(&line)
}

pub fn choose_user<R : Rng>(model: &Model, rng: &mut R) -> UserId {
    let user_ids : Vec<UserId> = model.users.keys().cloned().collect();
    let user_idx = rng.gen_range(0, user_ids.len());
    user_ids[user_idx]
} 

// pub fn generate<R : Rng>(model:&Model, rng: &mut R, sentence_start:&Vec<Token>, prev_line: &Option<Vec<Token>>) -> (UserId, String) {
//     let user_ids : Vec<UserId> = model.users.keys().cloned().collect();
//     let user_idx = rng.gen_range(0, user_ids.len());
//     let user_id = user_ids[user_idx];

//     println!("generating user_ids {:?} user_idx {} user_id {}", user_ids, user_idx, user_id);
//     generate_for_user(model, rng, sentence_start, user_id, prev_line)
// }

pub fn format_token(token:&Token) -> String {
    format!("{}", token)
}

pub fn format_selection(selection: &Option<GeneratedToken>, all_tokens:&Vec<Token>) -> String {
    if let &Some(ref generated_token) = selection {
        let token = all_tokens[generated_token.token_idx].clone();

        let populars : Vec<String> = generated_token.popular.iter().take(3).map(|&(token_idx, count)| {
            let t = all_tokens[token_idx].clone();
            format!("({:5}) {:12} ", count, t.to_string())
        }).collect();

        format!("{:5} ||| ({:5}) {:12} ||| {}", generated_token.table_occurrences, generated_token.chosen_occurrences, token.to_string(), populars.join(" "))
    } else {
        String::from(".")
    }
}


#[derive(Clone)]
pub struct GeneratedToken {
    pub token_idx: TokenIdx,
    pub chosen_occurrences: usize,
    pub table_occurrences: usize,
    
    pub popular: Vec<(TokenIdx, usize)>,
}

impl<C> GenerativeModel<C> where C: Eq + Hash + Copy {
    pub fn generate<R : Rng>(&self, current:&Line, idx:usize, token_map:&HashMap<Token, usize>, rng: &mut R) -> Option<GeneratedToken> {
        let cp = self.context_production;
        cp(current, idx, token_map).and_then(|context| {
            self.context_map.get(&context).map(|table| {
                let (token_idx, token_count) = select_from(table, rng);
                GeneratedToken {
                    token_idx: token_idx,
                    chosen_occurrences: token_count,
                    table_occurrences: table.occurences,
                    popular: most_popular(table, 3),
                }
            })
        })
    }
}

pub fn select_from<R : Rng>(table:&CumulativeWordTable, rng: &mut R) -> (TokenIdx, usize) {
    let n = rng.gen_range(0, table.occurences);
    let mut last_occur : OccurenceCount = 0;
    for i in 0..table.occurences {
        let (idx, occur) = table.token_table[i];
        let this_occur = occur - last_occur;
        if n < occur {
            return (idx, this_occur)
        }
        last_occur = occur;
    }
    (0, 0)
}

pub fn most_popular(table:&CumulativeWordTable, n:usize) -> Vec<(TokenIdx, usize)> {
    let mut out:Vec<(TokenIdx, usize)> = Vec::new();
    let mut last_occur : OccurenceCount = 0;
    
    for &(token_idx, occur) in table.token_table.iter().take(n) {
        let i_count = occur - last_occur;
        out.push((token_idx, i_count));
        last_occur = occur;
    } 

    out
}

pub fn generate_sentence(tokens:&Vec<Token>) -> String {
    use super::tokenizer::Token::*;

    let mut message = String::new();

    let trail = tokens.iter();
    let next = tokens.iter().skip(1);

    for (t, n) in trail.zip(next) {
        match (t, n) {
            (&Start, _) => (),
            (&End, _) => (),
            (&Word(ref word), &Word(_)) | (&Word(ref word), &Link(_)) => { // words need a space betwene them
                message.push_str(word);
                message.push(' ');
                ()
            },
            (&Word(ref word), _) => message.push_str(word),
            (&Link(ref link), &Link(_)) | (&Link(ref link), &Word(_)) => {
                message.push_str(link);
                message.push(' ');
                ()
            }
            (&Link(ref link), _) => message.push_str(link),
            (&Punctuation(ref punc, whitespace), _) => {
                message.push_str(punc);
                if whitespace {
                    message.push(' ');
                }
                ()
            },
        }
    }

    message
}
