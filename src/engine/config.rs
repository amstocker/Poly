use serde::{Serialize, Deserialize};



#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
  pub states: Vec<StateConfig>,
  pub lenses: Vec<LensConfig>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StateConfig {
  pub label: String,
  pub actions: Vec<String>
}

#[derive(Serialize, Deserialize, Debug)]
pub enum LensTypeConfig {
  Category,
  Monad,
  Iso
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LensConfig {
  pub label: String,
  pub rules: Vec<RuleConfig>,
  
  #[serde(rename = "type")]
  pub lens_type: Option<LensTypeConfig>,
  pub states: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RuleConfig {
  pub from: Vec<String>,
  pub to: Vec<String>
}
