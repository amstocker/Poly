use serde::{Serialize, Deserialize};



#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
  pub interfaces: Vec<InterfaceConfig>,
  pub lenses: Vec<LensConfig>,
  pub diagram: DiagramConfig
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
  pub source: Domain,
  pub target: Domain,
  pub rules: Vec<RuleConfig>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RuleConfig {
  pub from: Vec<String>,
  pub to: Vec<String>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DiagramConfig {
  #[serde(rename = "where")]
  where_diagrams: Vec<DiagramConfig>,
  source: Domain,
  target: Domain
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Domain {
  Once {
    label: String
  },
  Exactly {
    iterations: usize,
    label: String
  },
  Any {
    label: String
  },
  Composition {
    first: Box<Domain>,
    then: Box<Domain>
  }
}