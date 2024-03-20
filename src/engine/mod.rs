pub mod base;
pub mod config;
pub mod domain;
pub mod handle;
pub mod label;
pub mod rule;
pub mod util;



pub trait Engine<In, Out> {
  fn transduce(&self, input: In) -> Result<Out, Out>;
}

pub trait Layer<In, Out> {
  type InnerIn;
  type InnerOut;
  type InnerEngine: Engine<Self::InnerIn, Self::InnerOut>;

  fn inner(&self) -> &Self::InnerEngine;
  fn translate(&self, input: In) -> Self::InnerIn;
  fn untranslate(&self, output: Self::InnerOut) -> Out;
}

impl<T, In, Out> Engine<In, Out> for T where T: Layer<In, Out> {
  fn transduce(&self, input: In) -> Result<Out, Out> {
    self.inner().transduce(self.translate(input))
      .map(|output| self.untranslate(output))
      .map_err(|output| self.untranslate(output))
  }
}

