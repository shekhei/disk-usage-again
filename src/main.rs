#![feature(trait_alias)]
#![feature(unboxed_closures, fn_traits)]

#[macro_use]
extern crate clap;
extern crate num_cpus;
#[macro_use]
extern crate nom;

use std::env;
mod app;

fn main() {
    let args: Vec<String> = env::args().collect();
    let args_ref: Vec<&str> = args.iter().map(|s| &**s).collect();
    println!("debug {:?}", args);
    match app::app(&args_ref, std::io::stdout(), std::io::stderr()) {
        Err(x) => std::process::exit(x),
        _ => ()
    };
}