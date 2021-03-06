#![feature(proc_macro_hygiene, decl_macro)]

extern crate chrono;
#[macro_use]
extern crate diesel;
extern crate dotenv;
#[macro_use]
extern crate rocket;
#[macro_use]
extern crate rocket_contrib;
#[macro_use]
extern crate serde_derive;
extern crate syntect;
extern crate uuid;

mod db;

use rocket_contrib::templates::Template;

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

pub mod highlighting {
    use syntect::easy::HighlightLines;
    use syntect::highlighting::{Color, ThemeSet};
    use syntect::html::{styled_line_to_highlighted_html, IncludeBackground};
    use syntect::parsing::SyntaxSet;
    use syntect::util::LinesWithEndings;

    pub struct Highlighter {
        syntax_set: SyntaxSet,
        theme_set: ThemeSet,
    }

    impl Highlighter {
        pub fn new() -> Self {
            Highlighter {
                syntax_set: SyntaxSet::load_defaults_newlines(),
                theme_set: ThemeSet::load_defaults(),
            }
        }

        pub fn get_syntaxes(&self) -> Vec<String> {
            self.syntax_set
                .syntaxes()
                .iter()
                .map(|syn| syn.name.clone())
                .collect()
        }

        pub fn highlighted(&self, s: &str, syntax: &str, theme: &str) -> String {
            use std::fmt::Write;

            let theme = &self.theme_set.themes[theme];
            let sd = self
                .syntax_set
                .find_syntax_by_name(syntax)
                .unwrap_or_else(|| self.syntax_set.find_syntax_by_name("Plain Text").unwrap());

            let mut highlighter = HighlightLines::new(sd, theme);
            let c = theme.settings.background.unwrap_or(Color::WHITE);
            let mut output = format!(
                r#"<pre class="contents" style="background-color:#{:02x}{:02x}{:02x}">"#,
                c.r, c.g, c.b
            );
            let mut line_number = 1;
            for line in LinesWithEndings::from(s) {
                let regions = highlighter.highlight(line, &self.syntax_set);
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
                line_number += 1;
            }
            output.push_str("</pre>");
            output
        }
    }
}

mod routes {
    use std::path::{Path, PathBuf};

    use chrono::{DateTime, Local};
    use rocket::request::{Form, State};
    use rocket::response::{NamedFile, Redirect};
    use rocket_contrib::templates::Template;
    use uuid::Uuid;

    use db::Conn;
    use highlighting;
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
    pub struct UserPaste {
        pub language: String,
        pub contents: String,
    }

    #[get("/")]
    pub fn index(highlighter: State<highlighting::Highlighter>) -> Template {
        let ctx = HomeContext {
            syntaxes: highlighter.get_syntaxes(),
        };
        Template::render("index", &ctx)
    }

    #[get("/login")]
    pub fn login_page() -> Template {
        Template::render("login", &())
    }

    #[get("/paste/<id>")]
    pub fn load_paste(id: String, db: Conn) -> Template {
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
    pub fn new_paste(
        paste_form: Form<UserPaste>,
        db: Conn,
        highlighter: State<highlighting::Highlighter>,
    ) -> Redirect {
        let paste_form = paste_form.into_inner();
        let new_id = Uuid::new_v4();

        let html = highlighter.highlighted(
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

        Redirect::to(uri!(load_paste: id = new_id.to_string()))
    }

    #[get("/<file..>")]
    pub fn files(file: PathBuf) -> Option<NamedFile> {
        NamedFile::open(Path::new("static/").join(file)).ok()
    }
}

use db::Conn;

fn main() {
    dotenv::dotenv().ok();

    rocket::ignite()
        .mount(
            "/",
            routes![
                routes::index,
                routes::login_page,
                routes::load_paste,
                routes::new_paste
            ],
        )
        .manage(highlighting::Highlighter::new())
        .mount("/static", routes![routes::files])
        .attach(Conn::fairing())
        .attach(Template::fairing())
        .launch();
}
