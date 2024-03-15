use serde::{Serialize, Deserialize};



#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
  pub interfaces: Vec<InterfaceConfig>,
  pub lenses: Vec<LensConfig>,
  pub diagram: Diagram
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InterfaceConfig {
  pub label: String,
  pub states: Vec<StateConfig>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StateConfig {
  pub label: String,
  pub actions: Vec<String>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LensConfig {
  pub label: String,
  pub source: String,
  pub target: String,
  pub rules: Vec<RuleConfig>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RuleConfig {
  pub from: Vec<String>,
  pub to: Vec<String>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Diagram {
  #[serde(rename = "where")]
  where_diagrams: Vec<Diagram>,
  source: Domain,
  target: Domain
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Domain {
  Exactly(String),
  Any(String)
}