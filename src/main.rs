#![feature(plugin, custom_derive, custom_attribute)]
#![plugin(rocket_codegen)]

extern crate chrono;
#[macro_use]
extern crate diesel;
extern crate dotenv;
#[macro_use]
extern crate lazy_static;
extern crate rocket;
extern crate rocket_contrib;
#[macro_use]
extern crate serde_derive;
extern crate syntect;
extern crate uuid;

mod db;

use rocket_contrib::Template;
use std::env;

mod models {
    use chrono::NaiveDateTime;

    use db::schema::*;

    #[derive(
        Queryable, Insertable, PartialEq, Eq, Clone, Identifiable, Associations, AsChangeset,
    )]
    #[primary_key(post_id)]
    #[table_name = "post"]
    pub struct Post {
        pub post_id: Vec<u8>,
        pub user_id: Option<i32>,
        pub created_date: NaiveDateTime,
        pub expires_date: Option<NaiveDateTime>,
        pub language: String,
        pub contents: Vec<u8>,
        pub rendered: String,
        pub deletion_token: Vec<u8>,
    }
}

mod operations {
    use diesel::result::QueryResult;
    use diesel::*;
    use uuid::Uuid;

    use db::schema::post::dsl::*;
    use db::Conn;
    use models::Post;

    pub fn insert_paste(new_post: &Post, db: &Conn) -> QueryResult<()> {
        insert_into(post).values(new_post).execute(&**db)?;
        Ok(())
    }

    pub fn get_paste(id: &Uuid, db: &Conn) -> QueryResult<Post> {
        post.filter(post_id.eq(id.as_bytes().to_vec()))
            .get_result(&**db)
    }
}

mod routes {
    use std::path::{Path, PathBuf};

    use chrono::{DateTime, Local};
    use rocket::request::Form;
    use rocket::response::{NamedFile, Redirect};
    use rocket_contrib::Template;
    use uuid::Uuid;

    use db::Conn;
    use models::Post;
    use operations;

    #[derive(Serialize)]
    struct HomeContext {
        pub syntaxes: Vec<String>,
    }

    #[derive(Serialize)]
    struct PasteContext {
        pub created_date: String,
        pub expires_date: Option<String>,
        pub contents: String,
    }

    #[derive(FromForm)]
    struct UserPaste {
        pub language: String,
        pub contents: String,
    }

    #[get("/")]
    fn index() -> Template {
        let ctx = HomeContext {
            syntaxes: highlighting::get_syntaxes(),
        };
        Template::render("index", &ctx)
    }

    #[get("/login")]
    fn login_page() -> Template {
        Template::render("login", &())
    }

    mod highlighting {
        use syntect::easy::HighlightLines;
        use syntect::highlighting::{Color, ThemeSet};
        use syntect::html::{styled_line_to_highlighted_html, IncludeBackground};
        use syntect::parsing::SyntaxSet;

        pub fn highlighted(s: &str, syntax: &str, theme: &str) -> String {
            SYNTAX_SET.with(|ss| {
                use std::fmt::Write;

                let theme = &THEME_SET.themes[theme];
                let sd = ss
                    .find_syntax_by_name(syntax)
                    .unwrap_or_else(|| ss.find_syntax_by_name("Plain Text").unwrap());

                let mut highlighter = HighlightLines::new(sd, theme);
                let c = theme.settings.background.unwrap_or(Color::WHITE);
                let mut output = format!(
                    r#"<pre class="contents" style="background-color:#{:02x}{:02x}{:02x}">"#,
                    c.r, c.g, c.b
                );
                let mut line_number = 1;
                for line in s.lines() {
                    let regions = highlighter.highlight(line, ss);
                    let html = styled_line_to_highlighted_html(
                        &regions[..],
                        IncludeBackground::IfDifferent(c),
                    );
                    write!(
                        output,
                        r##"<a id="L{}" href="#L{}" class="line"></a>"##,
                        line_number, line_number
                    )
                    .unwrap();
                    output.push_str(&html);
                    output.push_str("\n");
                    line_number += 1;
                }
                output.push_str("</pre>");
                output
            })
        }

        pub fn get_syntaxes() -> Vec<String> {
            SYNTAX_SET.with(|ss| ss.syntaxes().iter().map(|syn| syn.name.clone()).collect())
        }

        thread_local! {
            static SYNTAX_SET: SyntaxSet = {
                SyntaxSet::load_defaults_nonewlines()
            }
        }

        lazy_static! {
            static ref THEME_SET: ThemeSet = ThemeSet::load_defaults();
        }
    }

    #[get("/paste/<id>")]
    fn load_paste(id: String, db: Conn) -> Template {
        let post_id = Uuid::parse_str(&id).unwrap();
        let post = operations::get_paste(&post_id, &db).unwrap();

        let ctx = PasteContext {
            created_date: DateTime::<Local>::from_utc(post.created_date, *Local::now().offset())
                .to_rfc2822(),
            expires_date: post
                .expires_date
                .map(|e| DateTime::<Local>::from_utc(e, *Local::now().offset()).to_rfc2822()),
            contents: post.rendered,
        };
        Template::render("paste", &ctx)
    }

    #[post("/paste/new", data = "<paste_form>")]
    fn new_paste(paste_form: Form<UserPaste>, db: Conn) -> Redirect {
        let paste_form = paste_form.into_inner();
        let new_id = Uuid::new_v4();

        use self::highlighting::highlighted;
        let html = highlighted(
            &paste_form.contents,
            &paste_form.language,
            "base16-ocean.dark",
        );

        let new_post = Post {
            post_id: new_id.as_bytes().to_vec(),
            user_id: None,
            created_date: Local::now().naive_local(),
            expires_date: None,
            language: paste_form.language,
            contents: paste_form.contents.into_bytes(),
            rendered: html,
            deletion_token: Uuid::new_v4().as_bytes().to_vec(),
        };
        operations::insert_paste(&new_post, &db).unwrap();

        Redirect::to(&format!("/paste/{}", new_id))
    }

    #[get("/<file..>")]
    fn files(file: PathBuf) -> Option<NamedFile> {
        NamedFile::open(Path::new("static/").join(file)).ok()
    }
}

fn main() {
    dotenv::dotenv().ok();

    rocket::ignite()
        .manage(db::init_pool(
            &env::var("RUSTY_BIN_DATABASE_URL").unwrap_or_else(|_| String::from("rusty-bin.db")),
        ))
        .mount(
            "/",
            routes![
                routes::index,
                routes::login_page,
                routes::load_paste,
                routes::new_paste
            ],
        )
        .mount("/static", routes![routes::files])
        .attach(Template::fairing())
        .launch();
}
