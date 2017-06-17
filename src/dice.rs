

use nom::{digit, space};
use std::str;
use std::str::FromStr;

use rand::Rng;

named!(number<u64>, 
    map_res!(
      map_res!(
        ws!(digit),
        str::from_utf8
      ),
      FromStr::from_str
    )
);

named!(dice<(Option<u64>, u64)>, 
    do_parse!(
        many0!(space) >>
        count: opt!(number) >>
        alt!(tag!("d") | tag!("D")) >>
        magnitude : number >> 
        many0!(space) >>
        (count, magnitude)
    )
);

pub fn parse_dice(text:&str) -> Option<Dice> {
    use nom::IResult::*;
    match  dice(text.as_bytes()) {
        Done(_, dice_params) => {
            Dice::from(dice_params)
        },
        Error(_) => None,
        Incomplete(_) => None,
    }
}

#[derive(Eq, PartialEq, Clone, Debug, Copy)]
pub struct Dice {
    pub count: u64,
    pub magnitude: u64,
}

impl Dice {
    pub fn from(params: (Option<u64>, u64)) -> Option<Dice> {
        let (count, mag) = params;

        let dice = Dice { count : count.unwrap_or(1), magnitude: mag };

        if dice.valid() {
            Some(dice)
        } else {
            None
        }
    }


    pub fn to_string(&self) -> String {
        format!("{}d{}", self.count, self.magnitude)
    }

    pub fn valid(&self) -> bool {
        self.count > 0 && self.magnitude > 0 && self.count < 100
    }

    pub fn roll<R : Rng>(&self, rng: &mut R) -> Vec<u64> {
        use rand::distributions::range::Range;
        use rand::distributions::IndependentSample;

        let die = Range::new(1, self.magnitude + 1);

        let mut rolls = Vec::new();
        
        for _ in 0..self.count {
            rolls.push(die.ind_sample(rng));
        }

        rolls
    }   
}
