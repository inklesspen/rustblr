extern crate rusqlite;
extern crate clap;

use clap::{App, SubCommand, AppSettings};
mod sqlhaver;
mod consumer;
mod authorize;


fn main() {
    let conn = sqlhaver::connect();

    let matches = App::new("rustblr")
        .setting(AppSettings::SubcommandRequired)
        .subcommand(SubCommand::with_name("consumer")
            .about("Set OAuth consumer key and secret")
            .after_help("Changing consumer keys erases all existing access tokens.")
            .arg_from_usage("<key> 'Consumer Key'")
            .arg_from_usage("<secret> 'Consumer Secret'"))
        .subcommand(SubCommand::with_name("authorize")
            .about("Authorize with a Tumblr account"))
        .subcommand(SubCommand::with_name("status")
            .about("Display OAuth status"))
        .get_matches();

    match matches.subcommand() {
        ("consumer", Some(matches)) => {
            consumer::consumer(conn, matches);
        },
        ("authorize", _) => {
            authorize::authorize(conn);
        },
        ("status", _) => {
            println!("other stuff");
        }
        _ => {
            panic!("at the disco");
        }
    }
}
