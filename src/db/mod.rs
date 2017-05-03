use std::ops::Deref;

use r2d2;
use diesel::sqlite::SqliteConnection;
use r2d2_diesel::ConnectionManager;

use rocket::http::Status;
use rocket::request::{self, FromRequest};
use rocket::{Request, State, Outcome};

pub mod schema;

pub type Pool = r2d2::Pool<ConnectionManager<SqliteConnection>>;

pub fn init_pool(database_url: &str) -> Pool {
    let config = r2d2::Config::default();
    let manager = ConnectionManager::<SqliteConnection>::new(database_url);
    r2d2::Pool::new(config, manager).expect("db pool")
}

pub struct Conn(r2d2::PooledConnection<ConnectionManager<SqliteConnection>>);

impl Conn {
    pub fn new(inner: r2d2::PooledConnection<ConnectionManager<SqliteConnection>>) -> Self {
        Conn(inner)
    }
}

impl Deref for Conn {
    type Target = SqliteConnection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for Conn {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Conn, ()> {
        let pool = match <State<Pool> as FromRequest>::from_request(request) {
            Outcome::Success(pool) => pool,
            Outcome::Failure(e) => return Outcome::Failure(e),
            Outcome::Forward(_) => return Outcome::Forward(()),
        };

        match pool.get() {
            Ok(conn) => Outcome::Success(Conn(conn)),
            Err(_) => Outcome::Failure((Status::ServiceUnavailable, ())),
        }
    }
}
