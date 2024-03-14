pub mod base;
pub mod config;
pub mod domain;
pub mod label;
pub mod rule;
pub mod util;



pub trait Engine<'a, In, Out> {
  fn transduce(&'a self, input: In) -> Result<Out, Out>;
}


pub trait Middleware<'a, In, Out: 'a> {
  type InnerIn;
  type InnerOut;
  type InnerEngine: Engine<'a, Self::InnerIn, Self::InnerOut>;

  fn inner(&self) -> &Self::InnerEngine;
  fn translate(&self, input: In) -> Self::InnerIn;
  fn untranslate(&'a self, output: Self::InnerOut) -> Out;
}

impl<'a, T, In: 'a, Out: 'a> Engine<'a, In, Out> for T where T: Middleware<'a, In, Out> {
  fn transduce(&'a self, input: In) -> Result<Out, Out> {
    self.inner().transduce(self.translate(input))
      .map(|output| self.untranslate(output))
      .map_err(|output| self.untranslate(output))
  }
}

