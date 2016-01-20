extern crate clap;
extern crate rusqlite;
extern crate oauth_client as oauth;

use clap::ArgMatches;
use sqlhaver::exists_checker;
use std::io;
use self::oauth::Token;

const CHECK: &'static str = "select exists(select id from consumer)"; 
const SAME: &'static str = "select exists(select id from consumer where key = ?1 and secret = ?2)";

pub fn consumer(conn: rusqlite::Connection, matches: &ArgMatches) {
    let key = matches.value_of("key").unwrap();  // required
    let secret = matches.value_of("secret").unwrap();  // required

    let tx = conn.transaction().unwrap();
    let already_exists = conn.query_row(CHECK, &[], &exists_checker).unwrap();
    if already_exists {
        let is_same = conn.query_row(SAME, &[&key, &secret], &exists_checker).unwrap();
        if is_same {
            println!("Consumer key and secret already stored; exiting.");
            return;
        }
        println!("This will overwrite the existing key and secret. Press Enter to continue.");
        let mut guess = String::new();
        io::stdin().read_line(&mut guess)
            .ok()
            .expect("Failed to read line");
        conn.execute("DELETE FROM access", &[]).unwrap();
        conn.execute("DELETE FROM consumer", &[]).unwrap();
    }
    conn.execute("INSERT INTO consumer (key, secret) VALUES (?1, ?2)", &[&key, &secret]).unwrap();
    tx.commit().unwrap();
}

const QUERY: &'static str = "SELECT key, secret FROM consumer";

pub fn get_token(conn: &rusqlite::Connection) -> Option<Token> {
    conn.query_row(QUERY, &[], |row| {
        let key: String = row.get(0);
        let secret: String = row.get(1);
        Token::new(key, secret)
    }).ok()
}
