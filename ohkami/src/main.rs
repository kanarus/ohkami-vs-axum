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

fn main() {
    /// ref: https://github.com/TechEmpower/FrameworkBenchmarks/blob/38c565ebaa900b4db51c0425d11a6619a5615a79/frameworks/Rust/axum/src/server.rs
    async fn serve(o: Ohkami) -> std::io::Result<()> {
        let socket = tokio::net::TcpSocket::new_v4()?;

        socket.set_reuseport(true)?;
        socket.set_reuseaddr(true)?;
        socket.set_nodelay(true)?;

        socket.bind("0.0.0.0:8000".parse().unwrap())?;

        let listener = socket.listen(4096)?;

        println!("an Ohkami is howling on port 8000 !");

        o.howl(listener).await;

        Ok(())
    }

    // for _ in 1..num_cpus::get() {
    //     std::thread::spawn(|| {
    //         tokio::runtime::Builder::new_current_thread()
    //             .enable_all()
    //             .build()
    //             .unwrap()
    //             .block_on(async {serve(ohkami().await).await.expect("serve error")})
    //     });
    // }
    // tokio::runtime::Builder::new_current_thread()
    //     .enable_all()
    //     .build()
    //     .unwrap()
    //     .block_on(async {serve(ohkami().await).await.expect("serve error")});

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {serve(ohkami().await).await.expect("serve error")});
}

pub async fn ohkami() -> Ohkami {
    Ohkami::new((
        SetServer,
        Context::new(Postgres::new().await),
        "/json"     .GET(json_serialization),
        "/db"       .GET(single_database_query),
        "/queries"  .GET(multiple_database_query),
        "/fortunes" .GET(fortunes),
        "/update"   .GET(database_updates),
        "/plaintext".GET(plaintext),
    ))
}

async fn json_serialization() -> JSON<Message> {
    JSON(Message {
        message: "Hello, World!"
    })
}

async fn single_database_query(
    Context(db): Context<'_, Postgres>,
) -> JSON<World> {
    let world = db.select_random_world().await;
    JSON(world)
}

async fn multiple_database_query(
    Query(q): Query<WorldsMeta<'_>>,
    Context(db): Context<'_, Postgres>,
) -> JSON<Vec<World>> {
    let n = q.parse();
    let worlds = db.select_n_random_worlds(n).await;
    JSON(worlds)
}

async fn fortunes(
    Context(db): Context<'_, Postgres>,
) -> FortunesTemplate {
    let mut fortunes = db.select_all_fortunes().await;
    fortunes.push(Fortune {
        id:      0,
        message: String::from("Additional fortune added at request time."),
    });
    fortunes.sort_unstable_by(|a, b| str::cmp(&a.message, &b.message));
    FortunesTemplate { fortunes }
}

async fn database_updates(
    Query(q): Query<WorldsMeta<'_>>,
    Context(db): Context<'_, Postgres>,
) -> JSON<Vec<World>> {
    let n = q.parse();
    let worlds = db.update_randomnumbers_of_n_worlds(n).await;
    JSON(worlds)
}

async fn plaintext() -> &'static str {
    "Hello, World!"
}
