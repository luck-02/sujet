#![allow(dead_code)]
use serde_json::{from_str, from_value};
use std::collections::BTreeMap;

use simeis_data::crew::CrewMemberType;
use simeis_data::galaxy::scan::ScanResult;
use simeis_data::galaxy::station::StationId;
use simeis_data::market::MarketTx;
use simeis_data::player::PlayerId;
use simeis_data::ship::cargo::ShipCargo;
use simeis_data::ship::module::ShipModuleType;
use simeis_data::ship::navigation::TravelCost;
use simeis_data::ship::resources::{ExtractionInfo, Resource};
use simeis_data::ship::upgrade::ShipUpgrade;
use simeis_data::ship::Ship;
use simeis_data::syslog::SyslogEvent;
use std::path::PathBuf;

use crate::data::{Player, SpaceCoord, Station};
use crate::json::{get_float, get_json, get_string, get_unsigned};

#[derive(Clone, Debug)]
pub enum ApiError {
    NotFound,
    Error {
        errtype: String,
        message: String,
    },

    JsonNoKey {
        key: &'static str,
        data: serde_json::Value,
    },
    JsonWrongType {
        key: &'static str,
        data: serde_json::Value,
        exptype: &'static str,
    },
}

impl From<ureq::Error> for ApiError {
    fn from(value: ureq::Error) -> Self {
        match value {
            ureq::Error::StatusCode(404) => ApiError::NotFound,
            _ => panic!("Error while communicating with API: {value:?}"),
        }
    }
}

pub struct ApiClient {
    server: String,
    key: Option<String>,
}

impl ApiClient {
    pub fn init<T: ToString>(server: T) -> ApiClient {
        ApiClient {
            server: server.to_string(),
            key: None,
        }
    }

    fn get<T: ToString>(&self, path: T) -> Result<serde_json::Value, ApiError> {
        let mut qry = ureq::get(format!("{}{}", self.server, path.to_string()));
        if let Some(ref key) = self.key {
            qry = qry.query("key", key)
        }

        let body = qry
            .call()?
            .body_mut()
            .read_to_string()
            .expect("Error while reading reply from server");

        let data = from_str(&body).expect("Error loading JSON from reply");
        let serde_json::Value::Object(mut map) = data else {
            unreachable!("Data not a map");
        };

        let error = map
            .remove("error")
            .expect("no error key")
            .as_str()
            .expect("error not a string")
            .to_string();
        let data = serde_json::Value::Object(map);

        if error == "ok" {
            Ok(data)
        } else {
            let errtype = get_string(&data, "type")?;
            Err(ApiError::Error {
                errtype,
                message: error,
            })
        }
    }

    pub fn ping(&self) -> Result<bool, ApiError> {
        let got = self.get("/ping")?;
        Ok(get_string(&got, "ping")? == "pong")
    }

    pub fn new_player<T: ToString>(&mut self, name: T) -> Result<u16, ApiError> {
        let got = self.get(format!("/player/new/{}", name.to_string()))?;
        let id = get_unsigned(&got, "playerId")?;
        let key = get_string(&got, "key")?;
        self.key = Some(key);
        Ok(u16::try_from(id).unwrap())
    }

    pub fn get_player(&self, id: u16) -> Result<Player, ApiError> {
        let got = self.get(format!("/player/{id}"))?;
        let player = from_value(got).unwrap();
        Ok(player)
    }

    pub fn list_ship_can_buy(&self, station_id: StationId) -> Result<Vec<Ship>, ApiError> {
        let got = self.get(format!("/station/{station_id}/shipyard/list"))?;
        Ok(from_value(get_json(&got, "ships")?).unwrap())
    }

    pub fn buy_ship(&self, station_id: StationId, ship_id: u64) -> Result<(), ApiError> {
        let _ = self.get(format!("/station/{station_id}/shipyard/buy/{ship_id}"))?;
        Ok(())
    }

    pub fn get_ship(&self, ship_id: u64) -> Result<Ship, ApiError> {
        let got = self.get(format!("/ship/{ship_id}"))?;
        Ok(from_value(got).unwrap())
    }

    pub fn hire_crew(&self, station_id: StationId, ctype: CrewMemberType) -> Result<u32, ApiError> {
        let t = match ctype {
            CrewMemberType::Pilot => "pilot",
            CrewMemberType::Operator => "operator",
            CrewMemberType::Trader => "trader",
            CrewMemberType::Soldier => "soldier",
        };
        let got = self.get(format!("/station/{station_id}/crew/hire/{t}"))?;
        let id = get_unsigned(&got, "id")?;
        Ok(id as u32)
    }

    pub fn get_station_status(&self, station_id: StationId) -> Result<Station, ApiError> {
        let got = self.get(format!("/station/{station_id}"))?;
        Ok(serde_json::from_value(got).unwrap())
    }

    pub fn buy_crew_upgrade(
        &self,
        station_id: StationId,
        ship_id: u64,
        crew_id: u32,
    ) -> Result<(f64, u8), ApiError> {
        let got = self.get(format!(
            "/station/{station_id}/crew/upgrade/ship/{ship_id}/{crew_id}"
        ))?;
        let cost = get_float(&got, "cost")?;
        let rank = get_unsigned(&got, "new-rank")? as u8;
        Ok((cost, rank))
    }

    pub fn get_crew_upgrade_price(
        &self,
        station_id: StationId,
        ship_id: u64,
    ) -> Result<BTreeMap<u32, f64>, ApiError> {
        let got = self.get(format!("/station/{station_id}/crew/upgrade/ship/{ship_id}"))?;
        let map: BTreeMap<u64, serde_json::Value> = serde_json::from_value(got).unwrap();
        let mut res = BTreeMap::new();
        for (id, data) in map.into_iter() {
            let cost = get_float(&data, "price")?;
            res.insert(id as u32, cost);
        }
        Ok(res)
    }

    pub fn assign_trader(&self, station_id: StationId, trader_id: u32) -> Result<(), ApiError> {
        let _ = self.get(format!(
            "/station/{station_id}/crew/assign/{trader_id}/trading"
        ))?;
        Ok(())
    }

    pub fn upgrade_trader(&self, station_id: StationId) -> Result<(f64, u8), ApiError> {
        let got = self.get(format!("/station/{station_id}/crew/upgrade/trader"))?;
        let cost = get_float(&got, "cost")?;
        let rank = get_unsigned(&got, "new-rank")? as u8;
        Ok((cost, rank))
    }

    pub fn list_shop_ship_module(
        &self,
        station_id: StationId,
    ) -> Result<BTreeMap<ShipModuleType, f64>, ApiError> {
        let got = self.get(format!("/station/{station_id}/shop/modules"))?;
        Ok(serde_json::from_value(got).unwrap())
    }

    pub fn buy_ship_module(
        &self,
        station_id: StationId,
        ship_id: u64,
        module: &ShipModuleType,
    ) -> Result<u64, ApiError> {
        let module: &'static str = module.into();
        let got = self.get(format!(
            "/station/{station_id}/shop/modules/{ship_id}/buy/{module}"
        ))?;
        get_unsigned(&got, "id")
    }

    pub fn list_ship_module_upgrade(
        &self,
        station_id: StationId,
        ship_id: u64,
    ) -> Result<BTreeMap<u64, f64>, ApiError> {
        let got = self.get(format!(
            "/station/{station_id}/shop/modules/{ship_id}/upgrade"
        ))?;
        let map: BTreeMap<u64, serde_json::Value> = serde_json::from_value(got).unwrap();
        let mut res = BTreeMap::new();
        for (id, data) in map.into_iter() {
            let price = get_float(&data, "price")?;
            res.insert(id, price);
        }
        Ok(res)
    }

    pub fn buy_ship_module_upgrade(
        &self,
        station_id: StationId,
        ship_id: u64,
        mod_id: u64,
    ) -> Result<(f64, u8), ApiError> {
        let got = self.get(format!(
            "/station/{station_id}/shop/modules/{ship_id}/upgrade/{mod_id}"
        ))?;
        let cost = get_float(&got, "cost")?;
        let nrank = get_unsigned(&got, "new-rank")?;
        Ok((cost, nrank as u8))
    }

    pub fn assign_pilot(
        &self,
        station_id: StationId,
        ship_id: u64,
        crew_id: u32,
    ) -> Result<(), ApiError> {
        let _ = self.get(format!(
            "/station/{station_id}/crew/assign/{crew_id}/{ship_id}/pilot"
        ))?;
        Ok(())
    }

    pub fn assign_operator(
        &self,
        station_id: StationId,
        crew_id: u32,
        ship_id: u64,
        mod_id: u64,
    ) -> Result<(), ApiError> {
        let _ = self.get(format!(
            "/station/{station_id}/crew/assign/{crew_id}/{ship_id}/{mod_id}"
        ))?;

        Ok(())
    }

    pub fn station_scan(&self, station_id: StationId) -> Result<ScanResult, ApiError> {
        let got = self.get(format!("/station/{station_id}/scan"))?;
        Ok(serde_json::from_value(got).unwrap())
    }

    pub fn travel_cost(&self, ship_id: u64, coord: SpaceCoord) -> Result<TravelCost, ApiError> {
        let (x, y, z) = coord;
        let got = self.get(format!("/ship/{ship_id}/travelcost/{x}/{y}/{z}"))?;
        Ok(serde_json::from_value(got).unwrap())
    }

    pub fn navigate(&self, ship_id: u64, coord: SpaceCoord) -> Result<TravelCost, ApiError> {
        let (x, y, z) = coord;
        let got = self.get(format!("/ship/{ship_id}/navigate/{x}/{y}/{z}"))?;
        Ok(serde_json::from_value(got).unwrap())
    }

    pub fn start_extraction(&self, ship_id: u64) -> Result<ExtractionInfo, ApiError> {
        let got = self.get(format!("/ship/{ship_id}/extraction/start"))?;
        Ok(serde_json::from_value(got).unwrap())
    }

    pub fn stop_extraction(&self, ship_id: u64) -> Result<(), ApiError> {
        let _ = self.get(format!("/ship/{ship_id}/extraction/stop"))?;
        Ok(())
    }

    pub fn resource_prices(&self) -> Result<BTreeMap<Resource, f64>, ApiError> {
        let got = self.get(format!("/market/prices"))?;
        let data = serde_json::from_value(get_json(&got, "prices")?).unwrap();
        Ok(data)
    }

    pub fn buy_resource(
        &self,
        station_id: StationId,
        resource: &Resource,
        qty: f64,
    ) -> Result<MarketTx, ApiError> {
        let resource: &'static str = resource.into();
        let got = self.get(format!("/market/{station_id}/buy/{}/{qty}", resource))?;
        Ok(serde_json::from_value(got).unwrap())
    }

    pub fn sell_resource(
        &self,
        station_id: StationId,
        resource: &Resource,
        qty: f64,
    ) -> Result<MarketTx, ApiError> {
        let resource: &'static str = resource.into();
        let got = self.get(format!("/market/{station_id}/sell/{}/{qty}", resource))?;
        Ok(serde_json::from_value(got).unwrap())
    }

    pub fn get_fee_rate(&self, station_id: StationId) -> Result<f64, ApiError> {
        let got = self.get(format!("/market/{station_id}/fee_rate"))?;
        get_float(&got, "fee_rate")
    }

    pub fn unload_cargo(
        &self,
        ship_id: u64,
        resource: &Resource,
        qty: f64,
    ) -> Result<f64, ApiError> {
        let resource: &'static str = resource.into();
        let got = self.get(format!("/ship/{ship_id}/unload/{resource}/{qty}"))?;
        Ok(get_float(&got, "unloaded")?)
    }

    pub fn refuel_ship(&self, station_id: StationId, ship_id: u64) -> Result<f64, ApiError> {
        let got = self.get(format!("/station/{station_id}/refuel/{ship_id}"))?;
        get_float(&got, "added-fuel")
    }

    pub fn repair_ship(&self, station_id: StationId, ship_id: u64) -> Result<f64, ApiError> {
        let got = self.get(format!("/station/{station_id}/repair/{ship_id}"))?;
        get_float(&got, "added-hull")
    }

    pub fn list_ship_upgrades(
        &self,
        station_id: StationId,
    ) -> Result<BTreeMap<ShipUpgrade, (f64, String)>, ApiError> {
        let got = self.get(format!("/station/{station_id}/shipyard/upgrade"))?;
        let map: BTreeMap<ShipUpgrade, serde_json::Value> = from_value(got).unwrap();
        let mut result = BTreeMap::new();
        for (upgr, value) in map.into_iter() {
            let price = get_float(&value, "price")?;
            let descr = get_string(&value, "description")?;
            result.insert(upgr, (price, descr));
        }
        Ok(result)
    }

    pub fn buy_ship_upgrade(
        &self,
        station_id: StationId,
        ship_id: u64,
        upgrade: ShipUpgrade,
    ) -> Result<f64, ApiError> {
        let upgrade: &'static str = upgrade.into();
        let got = self.get(format!(
            "/station/{station_id}/shipyard/upgrade/{ship_id}/{upgrade}"
        ))?;
        get_float(&got, "cost")
    }

    pub fn get_station_upgrade_price(
        &self,
        station_id: StationId,
        key: &'static str,
    ) -> Result<f64, ApiError> {
        let got = self.get(format!("/station/{station_id}/upgrades"))?;
        get_float(&got, key)
    }

    pub fn buy_station_cargo(
        &self,
        station_id: StationId,
        amnt: u64,
    ) -> Result<ShipCargo, ApiError> {
        let got = self.get(format!("/station/{station_id}/shop/cargo/buy/{amnt}"))?;
        Ok(serde_json::from_value(got).unwrap())
    }

    pub fn get_syslogs(&self) -> Result<Vec<(f64, SyslogEvent)>, ApiError> {
        let got = self.get(format!("/syslogs"))?;
        let nb = get_unsigned(&got, "nb")?;
        let mut res = Vec::with_capacity(nb as usize);
        let data: Vec<serde_json::Value> =
            serde_json::from_value(get_json(&got, "events")?).unwrap();
        for ev in data {
            res.push((
                get_float(&ev, "timestamp")?,
                serde_json::from_value(get_json(&ev, "event")?).unwrap(),
            ));
        }
        Ok(res)
    }

    pub fn import_player<T: ToString>(&self, fname: T) -> (PlayerId, Option<String>) {
        let fname = PathBuf::from(fname.to_string());
        let data = std::fs::read(&fname).expect("Unable to read file");
        let json: serde_json::Value = serde_json::from_slice(&data).expect("Unable to load");
        let id = get_unsigned(&json, "id").unwrap();
        let key = get_string(&json, "key").unwrap();
        let key = if key == "none" { None } else { Some(key) };
        (id as PlayerId, key)
    }

    pub fn export_player<T: ToString>(&self, id: PlayerId, fname: T) {
        let fname = PathBuf::from(fname.to_string());
        let json = serde_json::json!({
            "id": id,
            "key": if let Some(ref k) = self.key { k } else { "none" },
        })
        .to_string();
        std::fs::write(&fname, json).expect("Unable to export player");
    }
}
