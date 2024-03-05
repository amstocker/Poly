use serde::{Serialize, Deserialize};


#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
  pub states: Vec<StateConfig>,
  pub transforms: Vec<TransformConfig>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StateConfig {
  pub label: String,
  pub actions: Vec<String>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TransformConfig {
  pub from: Vec<String>,
  pub to: Vec<String>
}