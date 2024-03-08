use crate::engine::config::LensTypeConfig;


#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Lens {

}

pub enum LensType {
  Category,
  Monad,
  Iso,
  Other
}

impl From<Option<LensTypeConfig>> for LensType {
  fn from(value: Option<LensTypeConfig>) -> Self {
    match value {
      Some(group_type) => match group_type {
        LensTypeConfig::Category => LensType::Category,
        LensTypeConfig::Monad => LensType::Monad,
        LensTypeConfig::Iso => LensType::Iso
      },
      None => LensType::Other
    }
  }
}

#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub struct Rule<T> {
  pub from: T,
  pub to: T
}

