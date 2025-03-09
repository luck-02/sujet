use crate::api::ApiClient;
use crate::data::Player;
use crate::interface::*;
use ::ratatui::buffer::Buffer;
use ::ratatui::layout::Rect;
use ::simeis_data::crew::CrewMemberType;
use ::simeis_data::syslog::SyslogEvent;

pub struct Game {
    n: f64,
    ntot: f64,
    screen: usize,
    player: Player,
    #[allow(dead_code)]
    money: f64,
    #[allow(dead_code)]
    api: ApiClient,
}

impl Game {
    pub fn init() -> Game {
        let mut api: ApiClient = ApiClient::init("http://localhost:8080");
        let player_id: u16 = api.new_player("Luc11").unwrap();
        let player = api.get_player(player_id).unwrap();
        let money = player.money.unwrap();

        Game {
            n: 0.0,
            ntot: 50.0,
            screen: 0,
            player,
            money,
            api,
        }
    }

    pub fn messages(&self) -> Vec<(f64, SyslogEvent)> {
        return self.api.get_syslogs().unwrap();
    }

    pub fn get_ship_id(&self, station_id: &u16) -> u64 {
        let allships = (&self.api).list_ship_can_buy(*station_id).unwrap();
        let mut ship_id: u64 = 0;
        for ship in allships {
            let price = ship.compute_price();
            if price < self.money {
                ship_id = ship.id;
            }
        }
        return ship_id;
    }

    pub fn get_pilote(&self, station_id: &u16) -> Result<u32, crate::api::ApiError> {
        return self.api.hire_crew(*station_id, CrewMemberType::Pilot);
    }

    pub fn get_operator(&self, station_id: &u16) -> Result<u32, crate::api::ApiError> {
        return self.api.hire_crew(*station_id, CrewMemberType::Operator);
    }

    pub fn get_soldier(&self, station_id: &u16) -> Result<u32, crate::api::ApiError> {
        return self.api.hire_crew(*station_id, CrewMemberType::Soldier);
    }

    pub fn get_trader(&self, station_id: &u16) -> Result<u32, crate::api::ApiError> {
        return self.api.hire_crew(*station_id, CrewMemberType::Trader);
    }

    pub fn get_station_id(&self) -> &u16 {
        let station_id = (self.player.stations.keys()).nth(0).unwrap();
        return station_id;
    }

    pub fn buy_ship(&self, station_id: &u16) {
        let ship_id = self.get_ship_id(station_id);
        self.api.buy_ship(*station_id, ship_id).unwrap();
    }

    // Here, put all the code related to playing the game
    pub fn update(&mut self) -> bool {
        self.n = (self.n + 1.0) % self.ntot;
        false
    }

    // If you want to have multiple screens, and move around them,
    // the main loop will call `next_screen` when you press ->
    // and `prev_screen` when you press `<-`
    pub fn next_screen(&mut self) {
        self.screen = (self.screen + 1) % 2;
    }

    // and `prev_screen` when you press `<-`
    pub fn prev_screen(&mut self) {
        if self.screen == 0 {
            self.screen = 1;
        } else {
            self.screen -= 1;
        }
    }

    // This will call the function for the given screen to render
    pub fn render(&self, buffer: &mut Buffer, area: Rect) {
        if self.screen == 0 {
            self.screen_0(buffer, area)
        } else if self.screen == 1 {
            self.screen_1(buffer, area)
        } else {
            unreachable!("Screen {} doesn't exist", self.screen);
        }
    }

    // Here, put all the code related to the display of data on the screen
    pub fn screen_0(&self, buffer: &mut Buffer, area: Rect) {
        // Split the screen to 6 rectangles of equal value
        // Returns an array of 6 elements
        let areas = split_vertical::<6>(area);

        // Split the screen to 3 rectanges of equal value, horizontally
        let top = split_horizontal::<3>(areas[0]);
        text(top[0], buffer, format!("N = {}", self.n));
        text(
            top[1],
            buffer,
            format!("Nom du joueur = {}", self.player.name),
        );

        // Creates a bar chart with the given value for the given labels
        // Asks for a list of (String, f64) elements
        // let carburant = self.player.ships.;
        let data = vec![
            ("Argent".to_string(), self.player.money.unwrap()),
            // ("fuel_tank".to_string(), self.player.),
            ("Stations".to_string(), self.player.stations.len() as f64),
            // (
            //     "Carburant".to_string(),self.player.stations[0] as u,
            // ),
            ("crew".to_string(), 480.0),
        ];

        // for v in self.player.ships.iter() {
        //     data.push(("Vaisseaux".to_string(), v. as f64));
        //     // println!("{}", v[1]);
        // }
        vertical_barchart(top[2], buffer, "Informations globales", &data);

        // Creates 5 progress bars on the bottom areas of the screen
        // With the given title, and uses some internal variables to set the progress
        for n in 1..=5 {
            progress_bar(
                areas[n],
                buffer,
                format!("N{n}").as_str(),
                self.n,
                self.ntot,
            );
        }
    }

    pub fn screen_1(&self, buffer: &mut Buffer, area: Rect) {
        // text(area, buffer, "Screen 1")
        let messages = self.messages();
        for message in messages {
            text(area, buffer, message.0.to_string());
        }
    }
}
