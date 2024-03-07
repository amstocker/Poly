use serde::{Serialize, Deserialize};



#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
  pub states: Vec<StateConfig>,
  pub groups: Vec<GroupConfig>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StateConfig {
  pub label: String,
  pub actions: Vec<String>
}

#[derive(Serialize, Deserialize, Debug)]
pub enum GroupTypeConfig {
  Category,
  Monad,
  Iso
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GroupConfig {
  pub label: String,
  #[serde(rename = "type")]
  pub group_type: Option<GroupTypeConfig>,
  pub states: Option<Vec<String>>,
  pub rules: Vec<RuleConfig>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RuleConfig {
  pub from: Vec<String>,
  pub to: Vec<String>
}