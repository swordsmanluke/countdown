mod countdown;

use std::env::args;
use std::error::Error;
use crate::countdown::{CountdownCommand, Counterdowner, CountdownStore};

#[macro_use] extern crate lazy_static;
extern crate regex;


fn main() -> Result<(), Box<dyn Error>>{
    let mut args = args();
    args.next(); // Pop the terminal command off the stack. args 1+ are what we're interested in.

    let cmd= CountdownCommand::from(args);
    let mut ctr = Counterdowner::new(Box::new(CountdownStore{}));
    let out = ctr.execute_countdown(cmd)?;
    print!("{}", out);
    Ok(())
}
