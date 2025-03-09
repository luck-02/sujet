// TODO Copy the files instead of symlink
mod api;
mod data;
mod game;
mod interface;
mod json;
use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::DefaultTerminal;
use std::time::{Duration, Instant};

const LOOP_TIME: Duration = Duration::from_millis(50);
const UPD_TIME: Duration = Duration::from_millis(500);

fn run(mut terminal: DefaultTerminal) -> Result<()> {
    let mut game = game::Game::init();
    let stat_id = game.get_station_id();
    game.buy_ship(stat_id);
    let operator = (&game).get_operator(stat_id);
    let pilote = (&game).get_pilote(stat_id);
    let soldier = (&game).get_soldier(stat_id);
    let trader = (&game).get_trader(stat_id);
    game.messages();

    let mut last_update = Instant::now();
    'main: loop {
        let start = Instant::now();
        if last_update.elapsed() >= UPD_TIME {
            if game.update() {
                break Ok(());
            }
            last_update = Instant::now();
        }

        terminal.draw(|f| {
            let area = f.area();
            let buffer = f.buffer_mut();
            game.render(buffer, area)
        })?;

        let took = start.elapsed();
        let mut tpoll = LOOP_TIME
            .saturating_sub(took)
            .max(Duration::from_millis(10));

        while start.elapsed() < LOOP_TIME {
            if event::poll(tpoll)? {
                if let Event::Key(ev) = event::read()? {
                    match ev.code {
                        KeyCode::Char('q') => break 'main Ok(()),
                        KeyCode::Right => game.next_screen(),
                        KeyCode::Left => game.prev_screen(),
                        _ => {}
                    }
                }
            }
            tpoll = LOOP_TIME.saturating_sub(start.elapsed());
        }
    }
}

// fn init_game() {
//     let mut api: ApiClient = ApiClient::init("http://localhost:8080");
//     let player_id: u16 = api.new_player("Lucas").unwrap();
//     let player = api.get_player(player_id).unwrap();
//     let money = player.money.unwrap();
//     let station_id = player.stations.keys().nth(0).unwrap();
//     let allships = api.list_ship_can_buy(*station_id).unwrap();

//     for ship in allships {
//         let price = ship.compute_price();
//         if price < money {
//             api.buy_ship(*sid, ship.id).unwrap();
//             break;
//         }
//     }
// }

fn main() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = run(terminal);
    ratatui::restore();
    result
}
