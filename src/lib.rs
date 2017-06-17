#![crate_name="robbot"]
#![allow(dead_code)]

extern crate telegram_bot;
extern crate rustc_serialize;
extern crate chrono;
extern crate fnv;
extern crate glob;
extern crate rand;
#[macro_use]
extern crate nom;
#[macro_use]
extern crate lazy_static;

pub mod persistence;
pub mod model;
pub mod history;
pub mod tokenizer;
pub mod generate;
pub mod command;
pub mod search;
pub mod bot;
pub mod dice;

use fnv::FnvHasher;
use std::collections::{HashMap as StdHashMap, HashSet as StdHashSet};
use std::hash::BuildHasherDefault;


pub type HashMap<K, V> = StdHashMap<K, V, BuildHasherDefault<FnvHasher>>;
pub type HashSet<V> = StdHashSet<V, BuildHasherDefault<FnvHasher>>;

use rand::{Rng, XorShiftRng, SeedableRng};

pub fn unseeded_rng() -> XorShiftRng {
    let mut threaded_rng = rand::thread_rng();
    let random_seed = [threaded_rng.next_u32(), threaded_rng.next_u32(), threaded_rng.next_u32(), threaded_rng.next_u32()];
    // let manual_seed = [1_u32, 2, 3, 4];
    rand::XorShiftRng::from_seed(random_seed)
}