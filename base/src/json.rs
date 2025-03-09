use serde_json::{Number, Value};

use crate::api::ApiError;

pub fn get_json(data: &Value, key: &'static str) -> Result<Value, ApiError> {
    let keys = key.split(".").collect::<Vec<&'static str>>();
    let err = ApiError::JsonNoKey {
        key,
        data: data.clone(),
    };
    let Value::Object(map) = data else {
        return Err(err);
    };

    let key_tot = keys.len();
    let mut data = map;
    for (nk, key) in keys.into_iter().enumerate() {
        if nk == (key_tot - 1) {
            return data.get(key).cloned().ok_or(err);
        } else {
            let inner = data
                .get(key)
                .ok_or(err.clone())?
                .as_object()
                .ok_or(err.clone())?;
            data = inner;
        }
    }
    Err(err)
}

pub fn get_string(data: &Value, key: &'static str) -> Result<String, ApiError> {
    let val = get_json(data, key)?;
    let s = val.as_str().ok_or(ApiError::JsonWrongType {
        key,
        data: data.clone(),
        exptype: "string",
    })?;
    Ok(s.to_string())
}

fn get_number(data: &Value, key: &'static str, exp: &'static str) -> Result<Number, ApiError> {
    let val = get_json(data, key)?;
    let n = val.as_number().ok_or(ApiError::JsonWrongType {
        key,
        data: data.clone(),
        exptype: exp,
    })?;
    Ok(n.clone())
}

pub fn get_unsigned(data: &Value, key: &'static str) -> Result<u64, ApiError> {
    let n = get_number(data, key, "unsigned")?;
    let n = n.as_u64().ok_or(ApiError::JsonWrongType {
        key,
        data: data.clone(),
        exptype: "unsigned",
    })?;
    Ok(n)
}

pub fn get_float(data: &Value, key: &'static str) -> Result<f64, ApiError> {
    let n = get_number(data, key, "unsigned")?;
    let n = n.as_f64().ok_or(ApiError::JsonWrongType {
        key,
        data: data.clone(),
        exptype: "float",
    })?;
    Ok(n)
}
