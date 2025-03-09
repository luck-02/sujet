#![allow(dead_code, unused_imports)]
use std::collections::BTreeMap;

use serde::Deserialize;
use simeis_data::{crew::{Crew, CrewId, CrewMemberType}, galaxy::station::StationId, ship::{cargo::ShipCargo, module::ShipModule, Ship}};

pub type SpaceCoord = (u32, u32, u32);

#[derive(Deserialize, Debug)]
pub struct Player {
    pub id: u16,
    pub costs: Option<f64>,
    pub money: Option<f64>,
    pub name: String,
    pub ships: Option<Vec<Ship>>,
    pub stations: BTreeMap<StationId, SpaceCoord>,
}

impl Player {
    pub fn time_before_no_money(&self) -> std::time::Duration {
        std::time::Duration::from_secs_f64(self.money.unwrap() / self.costs.unwrap())
    }
}

#[derive(Deserialize, Debug)]
pub struct Station {
    pub id: StationId,
    pub position: SpaceCoord,
    pub crew: Crew,
    pub cargo: ShipCargo,
    pub idle_crew: Crew,
    pub trader: Option<CrewId>,
}
