use serde::{Deserialize, Serialize};

use crate::ship::resources::Resource;

use super::SpaceCoord;

// Informations that can be scanned from a planet
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PlanetInfo {
    pub position: SpaceCoord,
    pub temperature: u16,
    pub solid: bool,
}

impl PlanetInfo {
    pub fn scan(_rank: u8, planet: &Planet) -> PlanetInfo {
        PlanetInfo {
            position: planet.position,
            temperature: planet.temperature,
            solid: planet.solid,
        }
    }
}

pub struct Planet {
    pub position: SpaceCoord,
    temperature: u16,
    solid: bool,
}

impl Planet {
    pub fn random<R: rand::Rng>(coord: SpaceCoord, rng: &mut R) -> Planet {
        Planet {
            solid: rng.random_bool(0.4),
            temperature: rng.random(),
            position: coord,
        }
    }

    // TODO (#34) Make this depend on the conditions, temperature, etc...
    pub fn resource_density(&self, resource: &Resource) -> f64 {
        if self.solid {
            match resource {
                Resource::Stone => 3.0,
                Resource::Iron => 1.0,
                _ => 0.0,
            }
        } else {
            match resource {
                Resource::Helium => 3.0,
                Resource::Ozone => 1.0,
                _ => 0.0,
            }
        }
    }
}
