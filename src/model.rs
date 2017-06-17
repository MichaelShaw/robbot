
use super::HashMap;
use std::hash::Hash;
use std::fs::*;
use std::io::BufReader;
use std::io::BufRead;
use std::fmt;
use std::path::PathBuf;

use super::tokenizer::*;


pub type OccurenceCount = usize;
pub type TokenIdx = usize;
pub type UserId = u64;

pub type TokenMap<Context> = HashMap<Context, HashMap<TokenIdx, OccurenceCount>>;
pub type PackedTokenMap<Context> = HashMap<Context, CumulativeWordTable>;

pub type BigramContext = TokenIdx;
pub type TrigramContext = (TokenIdx, TokenIdx);

pub type BigramMap = TokenMap<BigramContext>;
pub type PackedBigramMap = PackedTokenMap<BigramContext>;

pub type TrigramMap = TokenMap<TrigramContext>;
pub type PackedTrigramMap = PackedTokenMap<TrigramContext>;

pub type Line = Vec<Token>;
pub type ContextF<F> = fn(&Line, usize, &HashMap<Token, usize>) -> Option<F>; 

pub struct LearningModel<C : Eq + Hash> {
    pub context_map: TokenMap<C>,
    pub context_production: ContextF<C>,
}

impl<C> fmt::Debug for LearningModel<C> where C : fmt::Debug + Eq + Hash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "LearningModel({:?})", self.context_map)
    }
}

impl<C> LearningModel<C> where C : Eq + Hash + Copy {
    pub fn ingest(&mut self, current:&Line, idx:usize, token_map:&HashMap<Token, usize>) {
        let current_token = *token_map.get(&current[idx]).unwrap();

        let cp = self.context_production;
        if let Some(c) = cp(current, idx, token_map) {
            increment_context_token(&mut self.context_map, c, current_token, 1);
        }
    }

    // this should only need a reference in theory
    pub fn as_generative(&self, min_count: usize) -> GenerativeModel<C> {
        let mut packed_map : PackedTokenMap<C> = HashMap::with_capacity_and_hasher(self.context_map.len(), Default::default()); // self.context_map.size()

        for (context, token_map) in &self.context_map {
            let table = pack_table(token_map);
            if table.occurences >= min_count {
                packed_map.insert(*context, table);
            }
       }

        let cp : ContextF<C> = self.context_production;
        
        GenerativeModel {
            context_map: packed_map,
            context_production: cp,
        }
    }
}

pub struct GenerativeModel<C : Eq + Hash> {
    pub context_map: PackedTokenMap<C>,
    pub context_production: ContextF<C>,
}

impl<C> fmt::Debug for GenerativeModel<C> where C: fmt::Debug + Eq + Hash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "GenerativeModel({:?})", self.context_map)
    }
}

#[derive(Debug)]
pub struct Model {
    pub token_to_idx : HashMap<Token, usize>,
    pub tokens : Vec<Token>,
    pub users : HashMap<UserId, UserGenerativeModel>,
    pub shared : UserGenerativeModel,
}

// just for temporary storage
#[derive(Debug)]
struct UserLearningModel {
    pub own_bigrams : LearningModel<BigramContext>, 
    pub own_trigrams : LearningModel<TrigramContext>,
}

fn add<C : Eq + Hash + Copy>(sink: &mut TokenMap<C>, from: &TokenMap<C>) {
    for (context, word_map) in from {
        for (token_idx, count) in word_map {
            increment_context_token(sink, *context, *token_idx, *count);
        }
    }
}

// fn new_hash_ma

fn increment_context_token<C : Hash + Eq + Copy>(map:&mut TokenMap<C>, c:C, idx:TokenIdx, n:OccurenceCount) {
    use std::collections::hash_map::Entry::*;

    let word_map = map.entry(c).or_insert_with(|| HashMap::default());

    match word_map.entry(idx) {
        Occupied(mut oe) => {
            *oe.get_mut() += n;
            ()
        },
        Vacant(ve) => {ve.insert(n);()},
    }
}

impl UserLearningModel {
    pub fn as_generative(&self, min_count: usize) -> UserGenerativeModel {
        UserGenerativeModel {
            own_bigrams: self.own_bigrams.as_generative(min_count),
            own_trigrams: self.own_trigrams.as_generative(min_count),
        }
    }

    pub fn add(&mut self, other: &UserLearningModel) {
        add(&mut self.own_bigrams.context_map, &other.own_bigrams.context_map);
        add(&mut self.own_trigrams.context_map, &other.own_trigrams.context_map);
    }
}

impl Default for UserLearningModel {
    fn default() -> UserLearningModel { 
        UserLearningModel {
            own_bigrams: LearningModel {
                context_map: HashMap::default(),
                context_production: independent_bigram_context,
            },
            own_trigrams: LearningModel {
                context_map: HashMap::default(),
                context_production: independent_trigram_context,
            },
        }
    }
}

#[derive(Debug)]
pub struct UserGenerativeModel {
    pub own_bigrams : GenerativeModel<BigramContext>, 
    pub own_trigrams : GenerativeModel<TrigramContext>,
}


impl UserGenerativeModel {
    pub fn relation_count(&self) -> usize {
        let mut count = 0;
        for (_, map) in &self.own_bigrams.context_map {
            count += map.token_table.len();
        }
        for (_, map) in &self.own_trigrams.context_map {
            count += map.token_table.len();
        }
        return count
    }
}

pub fn independent_bigram_context(current:&Line, idx:usize, token_to_idx: &HashMap<Token, usize>) -> Option<TokenIdx> {
    if idx > 0 {
        let prev_token = &current[idx-1];
        if interesting_token(prev_token) {
            token_to_idx.get(prev_token).map(|x| *x )
        } else {
            None
        }
    } else {
        None
    }
}

pub fn independent_trigram_context(current:&Line, idx:usize, token_to_idx: &HashMap<Token, usize>) -> Option<(TokenIdx, TokenIdx)> {
    if idx > 1 {
        let prev2 = &current[idx-2];
        let prev1 = &current[idx-1];
        if interesting_token(prev2) && interesting_token(prev1) {
            if let (Some(prev2_idx), Some(prev1_idx)) = (token_to_idx.get(prev2), token_to_idx.get(prev1)) {
                Some((*prev2_idx, *prev1_idx))
            } else {
                None
            } 
        } else {
            None
        }
    } else {
        None
    }
}

// generation friendly
#[derive(Debug)]
pub struct CumulativeWordTable {
    pub occurences: OccurenceCount,
    pub token_table: Vec<(TokenIdx, OccurenceCount)>,
}

pub fn pack_table(word_map:&HashMap<usize, OccurenceCount>) -> CumulativeWordTable {
    let mut pre_sort : Vec<(TokenIdx, OccurenceCount)> = Vec::with_capacity(word_map.len());

    for (token_idx, occur) in word_map {
        pre_sort.push((*token_idx, *occur));
    }
    // sort means common stuff is at front, for easy debugging + performance
    pre_sort.sort_by_key(|&(_,occur)| (occur as i32) * -1);

    let mut occurences = 0;
    let mut table : Vec<(TokenIdx, OccurenceCount)> = Vec::with_capacity(pre_sort.len());

    for (token_idx, occur) in pre_sort {
        occurences += occur;
        table.push((token_idx, occurences));
    }
    
    CumulativeWordTable { occurences: occurences, token_table:table }
}

pub fn interesting_token(token:&Token) -> bool {
    match token {
        &Token::Word(_) | &Token::Punctuation(_, _) | &Token::Link(_) | &Token::End | &Token::Start => true,
        // _ => false,
    }
}

pub fn leading_user_id(line:&str) -> Option<UserId> {
    let at = line.find(" ").unwrap();
    let (num, _) = line.split_at(at);
    num.parse().ok()
} 

pub fn parse_use_line(line:&str) -> (UserId, Vec<Token>) {
    let at = line.find(" ").unwrap();
    let (num, text) = line.split_at(at); 
    let user_id: UserId = num.parse().expect("parsing user_id");

    let tokens = tokenize_line(&text);

    (user_id, tokens)
}

pub fn create_models(paths:Vec<PathBuf>) -> Model {
    let mut user_models : HashMap<UserId, UserLearningModel> = HashMap::default();
    
    let mut token_map : HashMap<Token, usize> = HashMap::default();
    let mut all_tokens : Vec<Token> = Vec::new();

    
    for path in paths {
        // println!("opening path {:?}", path);
        let file = File::open(path).unwrap();
        let reader = BufReader::new(file);
        let lines = reader.lines();

        // let mut line_count = 0;

        for line_result in lines {
            // line_count += 1;
            let line = line_result.expect("attempted to read a line in model").to_lowercase();
            let (user_id, tokens) = parse_use_line(&line);

            // add token translation
            for t in &tokens {
                if !token_map.contains_key(&t) {
                    let next_idx = all_tokens.len();
                    token_map.insert(t.clone(), next_idx);
                    all_tokens.push(t.clone());
                }
            }

            // println!("line -> {:?}", tokens);

            let user_model = user_models.entry(user_id).or_insert_with(|| UserLearningModel::default());
            
            for idx in 0..tokens.len() {
                user_model.own_bigrams.ingest(&tokens, idx, &token_map);
                user_model.own_trigrams.ingest(&tokens, idx, &token_map);
            }
        }
        // println!("had {} lines", line_count);
    }

    let mut generative_user_models : HashMap<UserId, UserGenerativeModel> = HashMap::default();

    let mut shared_learning_model : UserLearningModel = UserLearningModel::default();

    println!("building generative models");

    for (user_id, learning_model) in user_models {
        println!("constructing generative for {} ...", user_id);
        generative_user_models.insert(user_id, learning_model.as_generative(0));
        println!("adding to shared ...");
        shared_learning_model.add(&learning_model);
        println!("done.");
    }


    println!("Building shared generative ...");
    let shared_generative = shared_learning_model.as_generative(0);
    println!("done.");

    Model {
        token_to_idx: token_map,
        tokens: all_tokens,
        users: generative_user_models,
        shared: shared_generative,
    }
} 
