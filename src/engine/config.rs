use serde::{Serialize, Deserialize};


#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
  pub states: Vec<StateConfig>,
  pub lenses: Vec<LensConfig>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StateConfig {
  pub label: String,
  pub actions: Vec<String>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LensConfig {
  pub source: String,
  pub target: String,
  pub delegations: Vec<DelegationConfig>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DelegationConfig {
  pub from: Vec<String>,
  pub to: Vec<String>
}