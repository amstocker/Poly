use crate::engine::config::GroupTypeConfig;


#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Group {

}

pub enum GroupType {
  Category,
  Monad,
  Iso,
  Other
}

impl From<Option<GroupTypeConfig>> for GroupType {
  fn from(value: Option<GroupTypeConfig>) -> Self {
    match value {
      Some(group_type) => match group_type {
        GroupTypeConfig::Category => GroupType::Category,
        GroupTypeConfig::Monad => GroupType::Monad,
        GroupTypeConfig::Iso => GroupType::Iso
      },
      None => GroupType::Other
    }
  }
}

#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub struct Rule<T> {
  pub from: T,
  pub to: T
}

