use std::collections::BTreeMap;

use cargo::ShipCargo;
use module::{ShipModule, ShipModuleId};
use navigation::{FlightData, Travel, TravelCost};
use rand::Rng;
use resources::{ExtractionInfo, Resource};
use serde::{Deserialize, Serialize};
use shipstats::ShipStats;

use crate::crew::{Crew, CrewId, CrewMemberType};
use crate::errors::Errcode;
use crate::galaxy::station::Station;
use crate::galaxy::{translation, Galaxy, SpaceCoord};

pub mod cargo;
pub mod module;
pub mod navigation;
pub mod resources;
pub mod shipstats;
pub mod upgrade;

const PILOT_FUEL_SHARE: u8 = 5; // Rank 10 = 4/5 fuel consumption
const HULL_USAGE_BASE: f64 = 5.0 / 100.0;

const FUEL_TANK_CAP_PRICE: f64 = 3.0;
const CARGO_CAP_PRICE: f64 = 10.0;
const HULL_DECAY_CAP_PRICE: f64 = 5.0;
const REACTOR_POWER_PRICE: f64 = 300.0;

const REACTOR_SPEED_PER_POWER: f64 = 50.0;

pub type ShipId = u64;

#[derive(Debug, Deserialize, Serialize, Default)]
pub enum ShipState {
    #[default]
    Idle,
    InFlight(FlightData),
    Extracting(ExtractionInfo),
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct Ship {
    pub id: ShipId,
    pub reactor_power: u16,
    pub fuel_tank_capacity: f64,
    pub hull_decay_capacity: f64,
    pub modules: BTreeMap<ShipModuleId, ShipModule>,

    #[serde(default)]
    pub position: SpaceCoord,
    #[serde(default)]
    pub crew: Crew,
    #[serde(default)]
    pub cargo: ShipCargo,
    #[serde(default)]
    pub fuel_tank: f64,
    #[serde(default)]
    pub hull_decay: f64,
    #[serde(default)]
    pub pilot: Option<CrewId>,
    #[serde(default)]
    pub state: ShipState,
    #[serde(default)]
    pub stats: shipstats::ShipStats,
}

impl Ship {
    pub fn init_shipyard(position: SpaceCoord) -> Vec<Ship> {
        let mut rng = rand::rng();
        vec![
            Ship::light(rng.random(), position),
            Ship::medium(rng.random(), position),
            Ship::heavy(rng.random(), position),
        ]
    }

    pub fn random(position: SpaceCoord) -> Ship {
        let mut rng = rand::rng();
        let cargo_cap = rng.random_range(100.0..10000.0) as f64;
        Ship {
            id: rng.random(),
            position,
            reactor_power: rng.random_range(1..10),
            fuel_tank_capacity: rng.random_range(1..10000) as f64,
            cargo: ShipCargo::with_capacity(cargo_cap),
            hull_decay_capacity: rng.random_range(1000..50000) as f64,
            ..Default::default()
        }
    }

    fn light(id: ShipId, position: SpaceCoord) -> Ship {
        Ship {
            id,
            position,
            reactor_power: 1,
            fuel_tank_capacity: 1000.0,
            cargo: ShipCargo::with_capacity(500.0),
            hull_decay_capacity: 3000.0,
            ..Default::default()
        }
    }

    fn medium(id: ShipId, position: SpaceCoord) -> Ship {
        Ship {
            id,
            position,
            reactor_power: 3,
            fuel_tank_capacity: 2000.0,
            cargo: ShipCargo::with_capacity(1000.0),
            hull_decay_capacity: 6000.0,
            ..Default::default()
        }
    }

    fn heavy(id: ShipId, position: SpaceCoord) -> Ship {
        Ship {
            id,
            position,
            reactor_power: 10,
            fuel_tank_capacity: 4000.0,
            cargo: ShipCargo::with_capacity(3000.0),
            hull_decay_capacity: 20000.0,
            ..Default::default()
        }
    }

    // TODO (#22) Create a new ship with random specs
    //         Used by traders to seek nice ships to buy

    // Public data of this ship to display on the marketplace
    pub fn market_data(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "price": self.compute_price(),
            "modules": self.modules,
            "reactor_power": self.reactor_power,
            "cargo_capacity": self.cargo.capacity,
            "fuel_tank_capacity": self.fuel_tank_capacity,
            "hull_decay_capacity": self.hull_decay_capacity,
        })
    }

    pub fn compute_price(&self) -> f64 {
        let mut price = 0.0;
        price += (self.reactor_power as f64) * REACTOR_POWER_PRICE;
        price += self.fuel_tank_capacity * FUEL_TANK_CAP_PRICE;
        price += self.cargo.capacity * CARGO_CAP_PRICE;
        price += self.hull_decay_capacity * HULL_DECAY_CAP_PRICE;
        price += self.modules.values().map(|m| m.totalcost).sum::<f64>();
        price
    }

    // Updates the performances of the ship based on the crew onboard
    pub fn update_perf_stats(&mut self) {
        self.stats = ShipStats::default();
        self.stats.hull_usage_rate = HULL_USAGE_BASE;
        self.stats.fuel_consumption = self.reactor_power as f64;

        if let Some(ref pilot) = self.pilot {
            let pilot = self.crew.0.get(pilot).unwrap();
            debug_assert!(matches!(pilot.member_type, CrewMemberType::Pilot));
            let totshare = (PILOT_FUEL_SHARE * 10) as f64;
            self.stats.fuel_consumption *= (totshare - (pilot.rank as f64)) / totshare;
            self.stats.speed =
                (self.reactor_power as f64) * REACTOR_SPEED_PER_POWER * (pilot.rank as f64);
        } else {
            self.stats.speed = 0.0;
        };
        self.stats.speed *= 1.0 - self.cargo.slowing_ratio();

        #[cfg(feature = "testing")]
        {
            self.stats.speed *= 100.0
        };
    }

    pub fn compute_travel_costs(&self, destination: SpaceCoord) -> Result<TravelCost, Errcode> {
        let ShipState::Idle = self.state else {
            return Err(Errcode::ShipNotIdle);
        };
        let travel = Travel::new(destination);
        let cost = travel.compute_costs(self)?;
        Ok(cost)
    }

    pub fn set_travel(&mut self, destination: SpaceCoord) -> Result<TravelCost, Errcode> {
        let ShipState::Idle = self.state else {
            return Err(Errcode::ShipNotIdle);
        };
        let travel = Travel::new(destination);
        let cost = travel.compute_costs(self)?;
        if !cost.have_enough(self) {
            return Err(Errcode::CannotPerformTravel);
        }
        log::debug!("Starting flight on ship {}", self.id);
        self.state = ShipState::InFlight(FlightData::new(self.position, &cost, &travel));
        Ok(cost)
    }

    pub fn update_flight(&mut self, mut tdelta: f64) -> bool {
        let ShipState::InFlight(ref mut data) = self.state else {
            unreachable!();
        };

        let mut finished = false;
        let mut dist_delta = self.stats.speed * tdelta;
        data.dist_done += dist_delta;
        if data.dist_done > data.dist_tot {
            let doverflow = data.dist_done - data.dist_tot;
            data.dist_done -= doverflow;
            dist_delta -= doverflow;
            let toverflow = doverflow / self.stats.speed;
            tdelta -= toverflow;
            finished = true;
        }

        self.fuel_tank -= self.stats.fuel_consumption * tdelta;
        if self.fuel_tank <= 0.0 {
            self.fuel_tank = 0.0;
            log::debug!("Ship {} has an empty fuel tank", self.id);
            return true;
        }

        self.hull_decay += self.stats.hull_usage_rate * dist_delta;
        if self.hull_decay >= self.hull_decay_capacity {
            log::debug!("Ship {} worn out all its hull", self.id);
            return true;
        }

        self.position = translation(data.start, data.direction, data.dist_done);

        if finished {
            debug_assert_eq!(self.position, data.destination);
        }
        finished
    }

    pub fn start_extraction(&mut self, galaxy: &Galaxy) -> Result<ExtractionInfo, Errcode> {
        let ShipState::Idle = self.state else {
            return Err(Errcode::ShipNotIdle);
        };
        let Some(planet) = galaxy.get_planet(&self.position) else {
            return Err(Errcode::CannotExtractWithoutPlanet);
        };
        log::debug!(
            "Ship {} started extraction on planet {:?}",
            self.id,
            planet.position
        );

        let extraction = ExtractionInfo::create(self, &planet);
        self.state = ShipState::Extracting(extraction.clone());
        log::debug!("Extraction of resources: {extraction:?}");
        Ok(extraction)
    }

    pub fn stop_extraction(&mut self) -> Result<(), Errcode> {
        let ShipState::Extracting(_) = self.state else {
            return Err(Errcode::ShipNotExtracting);
        };
        log::debug!("Ship {} stopped extraction", self.id);
        self.state = ShipState::Idle;
        Ok(())
    }

    pub fn update_extract(&mut self, tdelta: f64) -> bool {
        let ShipState::Extracting(ref rates) = self.state else {
            unreachable!();
        };
        rates.update_cargo(&mut self.cargo, tdelta)
    }

    pub fn unload_cargo(
        &mut self,
        resource: &Resource,
        amnt: f64,
        station: &mut Station,
    ) -> Result<f64, Errcode> {
        let unloaded = self.cargo.unload(resource, amnt);
        if unloaded == 0.0 {
            return Ok(0.0);
        }

        let added = station.cargo.add_resource(resource, unloaded);
        if added < unloaded {
            self.cargo.add_resource(resource, unloaded - added);
            Ok(added)
        } else {
            Ok(unloaded)
        }
    }
}
