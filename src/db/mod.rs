use diesel::SqliteConnection;

pub mod schema;

#[database("rusty_bin")]
pub struct Conn(SqliteConnection);
