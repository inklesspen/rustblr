extern crate rusqlite;

extern crate hyper;
extern crate curl;
extern crate oauth_client as oauth;
extern crate rand;
extern crate url;

use sqlhaver::exists_checker;
use std::io;
use std::io::Write;
use std::borrow::Cow;
use std::collections::HashMap;
use std::str;
use self::curl::http;
use self::curl::http::handle::Method;
use self::oauth::Token;
use self::url::Url;

mod api {
    pub const REQUEST_TOKEN: &'static str = "https://www.tumblr.com/oauth/request_token";
    pub const AUTHORIZE: &'static str = "https://www.tumblr.com/oauth/authorize";
    pub const ACCESS_TOKEN: &'static str = "https://www.tumblr.com/oauth/access_token";
    //pub const USER_INFO: &'static str = "https://api.tumblr.com/v2/user/info";
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
    let resp = http::handle()
                   .get(api::REQUEST_TOKEN)
                   .header("Authorization", header.as_ref())
                   .exec()
                   .unwrap();
    let resp = str::from_utf8(resp.get_body())
                   .unwrap()
                   .to_string();
    let param = split_query(resp.as_ref());
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
    let resp = http::handle()
                   .get(api::ACCESS_TOKEN)
                   .header("Authorization", header.as_ref())
                   .exec()
                   .unwrap();
    let resp = str::from_utf8(resp.get_body())
                   .unwrap()
                   .to_string();
    let param = split_query(resp.as_ref());
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

//fn get_user_info(consumer: &Token, access: &Token) {
    
    //let header = oauth::authorization_header(Method::Post, api::ECHO, consumer, Some(access), None);
//}

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

    //println!("consumer: {:?}", consumer_token);
    let request_token = get_request_token(&consumer_token);
    let verifier = authorize_request(&request_token).unwrap();
    let access_token = get_access_token(&consumer_token, &request_token, &verifier);
    conn.execute("INSERT INTO access (key, secret) VALUES (?1, ?2)", &[&access_token.key.to_string(), &access_token.secret.to_string()]).unwrap();
    tx.commit().unwrap();
}

