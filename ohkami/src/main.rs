mod fangs;
mod models;
mod postgres;
mod templates;

use {
    fangs::SetServer,
    models::Message,
    ohkami::prelude::*,
    ohkami::format::JSON,
};
use {
    models::{Fortune, World, WorldsMeta},
    postgres::Postgres,
    templates::FortunesTemplate,
    ohkami::format::Query,
};

#[tokio::main]
pub async fn main() {
    Postgres::init().await;
    
    Ohkami::new((
        SetServer,
        "/json"     .GET(json_serialization),
        "/db"       .GET(single_database_query),
        "/queries"  .GET(multiple_database_query),
        "/fortunes" .GET(fortunes),
        "/plaintext".GET(plaintext),
    )).howl("localhost:8000").await
}

async fn json_serialization() -> JSON<Message> {
    JSON(Message {
        message: "Hello, World!"
    })
}

async fn single_database_query(
    db: &Postgres,
) -> JSON<World> {
    let world = db.select_random_world().await;
    JSON(world)
}

async fn multiple_database_query(
    Query(q): Query<WorldsMeta<'_>>,
    db: &Postgres,
) -> JSON<Vec<World>> {
    let n = q.parse();
    let worlds = db.select_n_random_worlds(n).await;
    JSON(worlds)
}

async fn fortunes(
    db: &Postgres,
) -> FortunesTemplate {
    let mut fortunes = db.select_all_fortunes().await;
    fortunes.push(Fortune {
        id:      0,
        message: String::from("Additional fortune added at request time."),
    });
    fortunes.sort_unstable_by(|a, b| str::cmp(&a.message, &b.message));
    FortunesTemplate { fortunes }
}

/// This must not be used for actual benchmark.
/// See [`Postgres::update_randomnumbers_of_worlds`] for details.
#[allow(unused)]
async fn database_updates(
    Query(q): Query<WorldsMeta<'_>>,
    db: &Postgres,
) -> JSON<Vec<World>> {
    let n = q.parse();
    let worlds = db.update_randomnumbers_of_worlds(n).await;
    JSON(worlds)
}

async fn plaintext() -> &'static str {
    "Hello, World!"
}
