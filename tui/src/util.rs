pub trait TabState {
  fn name(&self) -> &str;
  fn index(&self) -> usize;
  fn next(&self) -> Self;
  fn prev(&self) -> Self;
}

pub struct TabsState<T> {
  state: T,
}

impl<T: TabState + Copy> TabsState<T> {
  pub fn new(state: T) -> TabsState<T> {
    Self { state }
  }

  pub fn state(&self) -> T {
    self.state
  }

  pub fn index(&self) -> usize {
    self.state.index()
  }

  pub fn next(&mut self) {
    self.state = self.state.next();
  }

  pub fn prev(&mut self) {
    self.state = self.state.prev();
  }
}
