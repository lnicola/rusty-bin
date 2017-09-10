#![feature(plugin, custom_derive, custom_attribute, conservative_impl_trait)]
#![plugin(rocket_codegen)]

extern crate chrono;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_codegen;
extern crate dotenv;
#[macro_use]
extern crate lazy_static;
extern crate r2d2;
extern crate r2d2_diesel;
extern crate rocket;
extern crate rocket_contrib;
#[macro_use]
extern crate serde_derive;
extern crate syntect;
extern crate uuid;

mod db;

use std::env;
use rocket_contrib::Template;

mod models {
    use chrono::NaiveDateTime;

    use db::schema::*;

    #[derive(Queryable, Insertable, PartialEq, Eq, Clone, Identifiable, Associations, AsChangeset)]
    #[primary_key(post_id)]
    #[table_name = "post"]
    pub struct Post {
        pub post_id: Vec<u8>,
        pub user_id: Option<i32>,
        pub created_date: NaiveDateTime,
        pub expires_date: Option<NaiveDateTime>,
        pub language: String,
        pub contents: Vec<u8>,
        pub deletion_token: Vec<u8>,
    }
}

mod operations {
    use diesel::*;
    use diesel::result::QueryResult;
    use uuid::Uuid;

    use db::Conn;
    use db::schema::post::dsl::*;
    use models::Post;

    pub fn insert_paste(new_post: &Post, db: &Conn) -> QueryResult<()> {
        insert(new_post).into(post).execute(&**db)?;
        Ok(())
    }

    pub fn get_paste(id: &Uuid, db: &Conn) -> QueryResult<Post> {
        post.filter(post_id.eq(id.as_bytes().to_vec()))
            .get_result(&**db)
    }
}

mod routes {
    use std::path::{Path, PathBuf};

    use rocket::request::Form;
    use rocket::response::{NamedFile, Redirect};
    use rocket_contrib::Template;
    use chrono::{DateTime, Local};
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
        pub wrapper_style: String,
        pub lines: Vec<String>,
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
        // let ctx = HomeContext {};
        Template::render("login", &())
    }

    mod highlighting {
        use syntect::parsing::{SyntaxDefinition, SyntaxSet};
        use syntect::highlighting::{self, Theme, ThemeSet};
        use syntect::html::{styles_to_coloured_html, IncludeBackground};
        use syntect::easy::HighlightLines;

        pub fn highlighted(s: &str, syntax: &str, theme: &str) -> (String, Vec<String>) {
            SYNTAX_SET.with(|ss| {
                let theme = &THEME_SET.themes[theme];
                let sd = ss.find_syntax_by_name(syntax).unwrap();

                highlighted_impl(s, sd, theme)
            })
        }

        pub fn get_syntaxes() -> Vec<String> {
            SYNTAX_SET.with(|ss| {
                ss.syntaxes().iter().map(|syn| syn.name.clone()).collect()
            })
        }

        fn highlighted_impl(
            s: &str,
            syntax: &SyntaxDefinition,
            theme: &Theme,
        ) -> (String, Vec<String>) {
            use std::fmt::Write;

            let mut output = String::new();
            let mut highlighter = HighlightLines::new(syntax, theme);
            let c = theme.settings.background.unwrap_or(highlighting::WHITE);
            let wrapper_style = format!("background-color:#{:02x}{:02x}{:02x}", c.r, c.g, c.b);
            write!(
                output,
                "<pre class=\"contents\" style=\"background-color:#{:02x}{:02x}{:02x};\">\n",
                c.r,
                c.g,
                c.b
            ).unwrap();

            let mut lines = Vec::new();
            let mut line_number = 1;
            for line in s.lines() {
                let regions = highlighter.highlight(line);
                let html = styles_to_coloured_html(&regions[..], IncludeBackground::IfDifferent(c));
                write!(
                    output,
                    "<a id=\"L{}\" href=\"#L{}\" class=\"line\"></a>{}",
                    line_number,
                    line_number,
                    html
                ).unwrap();
                output.push('\n');
                lines.push(html);
                line_number += 1;
            }
            output.push_str("</pre>\n");
            (wrapper_style, lines)
        }

        thread_local! {
            static SYNTAX_SET: SyntaxSet = {
                let mut ss = SyntaxSet::load_defaults_nonewlines();
                ss.link_syntaxes();
                ss
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

        let contents = String::from_utf8(post.contents).unwrap();

        use self::highlighting::highlighted;
        let (wrapper_style, lines) = highlighted(&contents, &post.language, "base16-ocean.dark");

        let ctx = PasteContext {
            created_date: DateTime::<Local>::from_utc(post.created_date, *Local::now().offset())
                .to_rfc2822(),
            expires_date: post.expires_date.map(|e| {
                DateTime::<Local>::from_utc(e, *Local::now().offset()).to_rfc2822()
            }),
            wrapper_style: wrapper_style,
            lines: lines,
        };
        Template::render("paste", &ctx)
    }

    #[post("/paste/new", data = "<paste_form>")]
    fn new_paste(paste_form: Form<UserPaste>, db: Conn) -> Redirect {
        let paste_form = paste_form.into_inner();
        let new_id = Uuid::new_v4();
        let new_post = Post {
            post_id: new_id.as_bytes().to_vec(),
            user_id: None,
            created_date: Local::now().naive_local(),
            expires_date: None,
            language: paste_form.language,
            contents: paste_form.contents.into_bytes(),
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
        .manage(db::init_pool(&env::var("RUSTY_BIN_DATABASE_URL")
            .unwrap_or_else(|_| String::from("rusty-bin.db"))))
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
