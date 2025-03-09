use ntex::web;

use simeis_data::game::Game;

mod api;
mod crew;
mod player;

pub type GameState = ntex::web::types::State<Game>;

#[ntex::main]
async fn main() -> std::io::Result<()> {
    #[cfg(not(feature = "testing"))]
    let port = 8080;

    #[cfg(feature = "testing")]
    let port = 9345;

    env_logger::builder()
        .parse_default_env()
        .filter_module("ntex_server", log::LevelFilter::Warn)
        .filter_module("ntex_io", log::LevelFilter::Warn)
        .filter_module("ntex_rt", log::LevelFilter::Warn)
        .filter_module("ntex::http::h1", log::LevelFilter::Warn)
        .init();
    log::info!("Running on http://127.0.0.1:{port}");
    let (gamethread, state) = Game::init();
    let game = state.clone();

    #[allow(clippy::redundant_closure)] // DEV
    let res = web::HttpServer::new(move || {
        web::App::new()
            .wrap(web::middleware::Logger::default())
            .state(state.clone())
            .configure(|srv| api::configure(srv))
    })
    .stop_runtime()
    .bind(("127.0.0.1", port))?
    .run()
    .await;

    game.stop(gamethread);
    res
}
