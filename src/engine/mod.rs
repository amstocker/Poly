pub mod lens;
pub mod config;

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use self::{config::*, lens::*};



#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Action {
  action: Entity,
  base_state: Entity
}

pub type Entity = usize;

pub fn new_entity() -> Entity {
  static ID_COUNTER: AtomicUsize = AtomicUsize::new(0);
  ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}



#[derive(Default)]
pub struct EntityMap(HashMap<Entity, String>);


impl EntityMap {
  pub fn insert(&mut self, label: String) -> Option<Entity> {
    let id = new_entity();
    match self.0.insert(id, label) {
      None => Some(id),
      Some(_) => None,
    }
  }

  pub fn resolve(&self, entity: Entity) -> Option<&String> {
    self.0.get(&entity)
  }

  pub fn resolve_all(
    &self,
    entities: impl IntoIterator<Item = Entity>
  ) -> impl Iterator<Item = Option<&String>> {
    entities.into_iter()
      .map(|entity| self.resolve(entity))
  }

  pub fn entities<'a, S: AsRef<str>>(
    &'a self,
    labels: impl IntoIterator<Item = S> + 'a
  ) -> impl Iterator<Item = Option<Entity>> + '_ {
    labels.into_iter()
      .map(|label| self.entity(label))
  }

  pub fn entity<S: AsRef<str>>(&self, label: S) -> Option<Entity> {
    self.0.iter()
      .find(|(_, v)| *v == label.as_ref())
      .map(|(&e, _)| e)
  }


}


#[derive(Default)]
pub struct Engine {
  entity_map: EntityMap,
  action_map: HashMap<Entity, Action>,
  pub lenses: Vec<Lens<Action>>
}

impl Engine {
  pub fn build_from_config(config: Config) -> Engine {
    let mut engine = Engine::default();
  
    for InterfaceConfig { states: state_configs, .. } in config.interfaces {
      for StateConfig { label, actions: action_labels } in state_configs {
        let state_entity = engine.entity_map.insert(label).unwrap();
        for label in action_labels {
          let action_entity = engine.entity_map.insert(label).unwrap();
          engine.action_map.insert(action_entity, Action { action: action_entity, base_state: state_entity });
        }
      }
    }

    for LensConfig { rules: rule_configs, .. } in config.lenses {
      let mut rules = Vec::new();
      for RuleConfig { from, to } in rule_configs {
        rules.push(Rule {
          from: engine.actions(from),
          to: engine.actions(to)
        });
      }
      engine.lenses.push(Lens::new(rules));
    }
  
    engine
  }

  fn actions<'a, S: AsRef<str>>(
    &'a self,
    labels: impl IntoIterator<Item = S> + 'a
  ) -> impl Iterator<Item = Action> + '_ {
    self.entity_map
      .entities(labels)
      .map(|entity| self.action_map.get(&entity.unwrap()).unwrap())
      .copied()
  }

  fn labels<'a>(
    &'a self, actions: impl IntoIterator<Item = Action> + 'a
  ) -> impl Iterator<Item = String> + 'a {
    self.entity_map
      .resolve_all(actions.into_iter().map(|action| action.action))
      .map(|s| s.unwrap())
      .cloned()
  }

  pub fn transduce<S: AsRef<str>>(
    &self,
    lens: &Lens<Action>,
    stack: impl IntoIterator<Item = S>
  ) -> Result<Vec<Vec<String>>, ()> {
    match lens.transduce(self.actions(stack).collect::<Vec<_>>().into()) {
      Ok(iter) =>
        Ok(iter.map(|stack| self.labels(stack).collect()).collect()),
      Err(_) => Err(()),
    }
  }
}


