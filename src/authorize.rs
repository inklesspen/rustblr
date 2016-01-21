extern crate rusqlite;

extern crate hyper;
extern crate curl;
extern crate oauth_client as oauth;
extern crate rand;
extern crate url;
extern crate serde_json;

use sqlhaver::exists_checker;
use std::io;
use std::io::Read;
use std::io::Write;
use std::borrow::Cow;
use std::collections::HashMap;
use self::curl::http::handle::Method;  // required by oauth_client :(
use self::oauth::Token;
use self::url::Url;
use self::serde_json::Value;

mod api {
    pub const REQUEST_TOKEN: &'static str = "https://www.tumblr.com/oauth/request_token";
    pub const AUTHORIZE: &'static str = "https://www.tumblr.com/oauth/authorize";
    pub const ACCESS_TOKEN: &'static str = "https://www.tumblr.com/oauth/access_token";
    pub const USER_INFO: &'static str = "https://api.tumblr.com/v2/user/info";
}

fn split_query<'a>(query: &'a str) -> HashMap<Cow<'a, str>, Cow<'a, str>> {
    let mut param = HashMap::new();
    for q in query.split('&') {
        let mut s = q.splitn(2, '=');
        let k = s.next().unwrap();
        let v = s.next().unwrap();
        let _ = param.insert(k.into(), v.into());
    }
    param
}

fn get_request_token(consumer: &Token) -> Token<'static> {
    let header = oauth::authorization_header(Method::Get, api::REQUEST_TOKEN, consumer, None, None);
    let mut resp = hyper::Client::new()
        .get(api::REQUEST_TOKEN)
        .header(hyper::header::Authorization(header.to_owned()))
        .send()
        .unwrap();
    let mut buf: String = String::new();
    resp.read_to_string(&mut buf).unwrap();
    let param = split_query(buf.as_ref());
    Token::new(param.get("oauth_token").unwrap().to_string(),
               param.get("oauth_token_secret").unwrap().to_string())
}

fn get_access_token(consumer: &Token, request: &Token, verifier: &str) -> Token<'static> {
    let mut param = HashMap::new();
    param.insert("oauth_verifier".into(), verifier.into());
    let header = oauth::authorization_header(Method::Get,
                                             api::ACCESS_TOKEN,
                                             consumer,
                                             Some(request),
                                             Some(&param));
    let mut resp = hyper::Client::new()
        .get(api::ACCESS_TOKEN)
        .header(hyper::header::Authorization(header.to_owned()))
        .send()
        .unwrap();
    let mut buf: String = String::new();
    resp.read_to_string(&mut buf).unwrap();
    let param = split_query(buf.as_ref());
    Token::new(param.get("oauth_token").unwrap().to_string(),
               param.get("oauth_token_secret").unwrap().to_string())
}

fn authorize_request(request: &Token) -> Option<String> {
    let mut url = Url::parse(api::AUTHORIZE).unwrap();
    let pairs = vec![("oauth_token", request.key.to_string())];
    url.set_query_from_pairs(pairs.into_iter());
    println!("Go to this URL: {}", url);
    print!("Enter verifier code: ");
    io::stdout().flush().unwrap();
    let mut verifier = String::new();
    io::stdin().read_line(&mut verifier).ok().expect("Did not get verifier");
    Some(verifier.trim().to_string())
}

pub fn authorize(conn: rusqlite::Connection) {
    let consumer_token = super::consumer::get_token(&conn).unwrap();

    let tx = conn.transaction().unwrap();
    let already_exists = conn.query_row("SELECT exists(SELECT id FROM access)", &[], &exists_checker).unwrap();
    if already_exists {
        println!("An access token is already stored. Proceeding will overwrite it. Press Enter to continue.");
        let mut guess = String::new();
        io::stdin().read_line(&mut guess)
            .ok()
            .expect("Failed to read line");
        conn.execute("DELETE FROM access", &[]).unwrap();
    }

    let request_token = get_request_token(&consumer_token);
    let verifier = authorize_request(&request_token).unwrap();
    let access_token = get_access_token(&consumer_token, &request_token, &verifier);
    conn.execute("INSERT INTO access (key, secret) VALUES (?1, ?2)", &[&access_token.key.to_string(), &access_token.secret.to_string()]).unwrap();
    tx.commit().unwrap();
}


const ACCESS_QUERY: &'static str = "SELECT key, secret FROM access";

pub fn load_access_token(conn: &rusqlite::Connection) -> Option<Token> {
    conn.query_row(ACCESS_QUERY, &[], |row| {
        let key: String = row.get(0);
        let secret: String = row.get(1);
        Token::new(key, secret)
    }).ok()
}

pub fn check_status(conn: rusqlite::Connection) {
    let consumer_token = super::consumer::get_token(&conn).unwrap();
    let access_token = load_access_token(&conn).unwrap();
    let header = oauth::authorization_header(Method::Get, api::USER_INFO, &consumer_token, Some(&access_token), None);
    let client = hyper::Client::new();
    let res = client.get(api::USER_INFO)
        .header(hyper::header::Authorization(header.to_owned()))
        .send().unwrap();
    match res.status {
        hyper::status::StatusCode::Ok => {
            let data: Value = serde_json::from_reader(res).unwrap();
            let username = data.lookup("response.user.name").and_then(|val| val.as_string()).unwrap();
            println!("Authorized. Username: {}", username);

        },
        hyper::status::StatusCode::Unauthorized => {
            println!("Unauthorized!")
        },
        _ => {
            println!("Got status of {:?}, which was unexpected.", res.status);
        }
    }
}
