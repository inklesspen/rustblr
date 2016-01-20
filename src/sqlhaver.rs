extern crate rusqlite;

use std::env;
use rusqlite::{Connection, Row, Result};

mod ddl {
    pub const CONSUMER: &'static str = "
              CREATE TABLE consumer (
                id INTEGER PRIMARY KEY,
                key VARCHAR,
                secret VARCHAR
              )";
    pub const ACCESS: &'static str = "
              CREATE TABLE access (
                id INTEGER PRIMARY KEY,
                key VARCHAR,
                secret VARCHAR
              )";
}

const TABLE_EXISTENCE: &'static str = "select exists(select name from sqlite_master where type='table' and name=?1)";

fn table_exists(conn: &Connection, table_name: &str) -> rusqlite::Result<bool>  {
    conn.query_row(TABLE_EXISTENCE, &[&table_name], &exists_checker)
}

pub fn exists_checker(row: Row) -> bool {
    let val: i64 = row.get(0);
    val == 1
}

fn make_conn() -> Connection {
    let home_dir = env::home_dir().expect("Unable to find home directory, bailing");
    let sqlite_path = home_dir.join(".rustblr.sqlite");
    let conn = Connection::open(sqlite_path).expect("Unable to open config file, bailing");
    conn
}

fn ensure_tables(conn: &Connection) -> rusqlite::Result<()> {
    let tx = try!(conn.transaction());
    if !try!(table_exists(&conn, "consumer")) {
        try!(conn.execute(ddl::CONSUMER, &[]));
    }
    if !try!(table_exists(&conn, "access")) {
        try!(conn.execute(ddl::ACCESS, &[]));
    }
    tx.commit()
}

pub fn connect() -> Connection {
    let conn = make_conn();
    ensure_tables(&conn).unwrap();
    conn
}
