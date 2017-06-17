extern crate robbot;
extern crate chrono;

use robbot::bot::*;
use std::time::{Instant, Duration};
use std::thread;
use std::cmp::min;

use std::env;

fn main() {
    let args : Vec<_> = env::args().collect();
    if let Some(key) = args.get(1) {
        let base_sleep_duration : u64 = 120;
        let mut sequential_fails = 0;

        let fail_duration = Duration::new(120, 0);

        println!("Building with key -> {:?}", key);
        let mut bot = Bot::build(key, "../chat", "../history").expect("a bot");
        println!("Entering main loop");
        
        'main : loop {
            println!("Starting");
            let pre_start = Instant::now();
            
            let res = bot.run();
            println!("Run result -> {:?}", res);

            let elapsed = pre_start.elapsed();
            if elapsed > fail_duration {
                println!("Duration is longer than fails, just going to retry");
                sequential_fails = 0;
            } else {
                sequential_fails += 1;
                let seconds_to_sleep = base_sleep_duration * 2u64.pow(min(10,sequential_fails));
                println!("Failure to start, {:?} fails, sleeping for {:?}", sequential_fails, seconds_to_sleep);
                thread::sleep(Duration::new(seconds_to_sleep, 0));
            }
        }
    } else {
        println!("Need to specify api key as arg")
    }
}
