use tui::backend::Backend;
use tui::Frame;
use tui::layout::{Constraint, Direction, Layout, Rect};

// Element

pub trait Element<'a, B: Backend, M> {
  fn render(&self, f: &mut Frame<B>, area: Rect, selected: bool);

  fn message(&mut self, message: M);
}

pub struct FnElement<FR, FM> {
  render_fn: FR,
  message_fn: FM,
}

impl<FR, FM> FnElement<FR, FM> {
  pub fn new<'a, B: Backend, M>(render_fn: FR, message_fn: FM) -> Self where FR: Fn(&mut Frame<B>, Rect, bool) + 'a, FM: FnMut(M) + 'a {
    Self { render_fn, message_fn }
  }
}

impl<'a, B: Backend, FR: Fn(&mut Frame<B>, Rect, bool) + 'a, M, FM: FnMut(M) + 'a> Element<'a, B, M> for FnElement<FR, FM> {
  fn render(&self, f: &mut Frame<B>, area: Rect, selected: bool) {
    (self.render_fn)(f, area, selected);
  }

  fn message(&mut self, message: M) {
    (self.message_fn)(message);
  }
}

// Container

enum ElementOrContainer<'a, B: Backend, M> {
  Element(Box<dyn Element<'a, B, M> + 'a>),
  Container(Container<'a, B, M>),
}

pub struct Container<'a, B: Backend, M> {
  items: Vec<ElementOrContainer<'a, B, M>>,
  direction: Direction,
  layout: Layout,
  selection_index: Option<usize>,
  last_selection_index: Option<usize>,
}

// Creation and addition

impl<'a, B: Backend, M> Container<'a, B, M> {
  pub fn new(direction: Direction) -> Container<'a, B, M> {
    Self {
      items: Vec::new(),
      direction: direction.clone(),
      layout: Layout::default().direction(direction),
      selection_index: None,
      last_selection_index: None,
    }
  }

  pub fn rows() -> Self {
    Self::new(Direction::Vertical)
  }

  pub fn cols() -> Self {
    Self::new(Direction::Horizontal)
  }


  pub fn constraints(mut self, constraints: impl Into<Vec<Constraint>>) -> Self {
    self.layout = self.layout.constraints(constraints);
    self
  }


  pub fn element(mut self, element: Box<dyn Element<'a, B, M>>) -> Self {
    self.items.push(ElementOrContainer::Element(element));
    self
  }

  pub fn widget<FR: Fn(&mut Frame<B>, Rect, bool) + 'a, FM: FnMut(M) + 'a>(mut self, render_fn: FR, message_fn: FM) -> Self {
    let element = FnElement::new(render_fn, message_fn);
    let b = Box::new(element);
    self.items.push(ElementOrContainer::Element(b));
    self
  }

  pub fn container(mut self, container: Container<'a, B, M>) -> Self {
    self.items.push(ElementOrContainer::Container(container));
    self
  }
}

// Rendering

impl<'a, B: Backend, M> Container<'a, B, M> {
  pub fn render(&self, f: &mut Frame<B>, area: Rect) {
    let chunks = self.layout.split(area);
    for (index, (item, area)) in self.items.iter().zip(chunks).enumerate() {
      match item {
        ElementOrContainer::Element(element) => {
          element.render(f, area, self.selection_index.map_or(false, |i| index == i))
        }
        ElementOrContainer::Container(container) => {
          container.render(f, area)
        }
      }
    }
  }
}

// Message handling

impl<'a, B: Backend, M> Container<'a, B, M> {
  pub fn message(&mut self, message: M) {
    for (index, item) in self.items.iter_mut().enumerate() {
      if self.selection_index.map_or(false, |i| index == i) {
        match item {
          ElementOrContainer::Element(element) => {
            element.message(message);
            break;
          }
          ElementOrContainer::Container(container) => {
            container.message(message);
            break;
          }
        }
      }
    }
  }
}

// Selection

impl<'a, B: Backend, M> Container<'a, B, M> {
  pub fn move_selection_up(&mut self) {
    self.move_selection(Direction::Horizontal, true);
  }

  pub fn move_selection_down(&mut self) {
    self.move_selection(Direction::Horizontal, false);
  }

  pub fn move_selection_left(&mut self) {
    self.move_selection(Direction::Vertical, true);
  }

  pub fn move_selection_right(&mut self) {
    self.move_selection(Direction::Vertical, false);
  }

  fn move_selection(
    &mut self,
    direction: Direction,
    up_left: bool, // true = up/left, false = down/right
  ) -> bool {
    if let Some(selection_index) = self.selection_index {
      if let Some(new_selection_index) = Self::new_selection_index(selection_index, up_left, self.items.len()) {
        if let Some(ElementOrContainer::Container(container)) = self.items.get_mut(selection_index) {
          container.unselect();
        }
        self.selection_index = Some(new_selection_index);
        self.last_selection_index = self.selection_index;
        let self_direction = self.direction.clone();
        if let Some(ElementOrContainer::Container(container)) = self.items.get_mut(new_selection_index) {
          let mode = Self::selection_change_mode(self_direction, direction.clone(), up_left);
          container.select(mode);
        }
      }
      true
    } else {
      for item in self.items.iter_mut() {
        if let ElementOrContainer::Container(container) = item {
          if container.move_selection(direction.clone(), up_left) {
            return true;
          }
        }
      }
      false
    }
  }

  fn new_selection_index(index: usize, up_left: bool, len: usize) -> Option<usize> {
    if up_left {
      if index < len {
        Some(index + 1)
      } else {
        None
      }
    } else {
      if index > 0 {
        Some(index - 1)
      } else {
        None
      }
    }
  }

  fn selection_change_mode(self_direction: Direction, direction: Direction, up_left: bool) -> SelectionChangeMode {
    if self_direction != direction {
      SelectionChangeMode::Restore
    } else if up_left {
      SelectionChangeMode::Max
    } else {
      SelectionChangeMode::Min
    }
  }

  fn unselect(&mut self) {
    self.last_selection_index = self.selection_index;
    self.selection_index = None;
  }

  fn select(&mut self, mode: SelectionChangeMode) {
    use SelectionChangeMode::*;
    self.selection_index = Some(match mode {
      Restore => self.last_selection_index.unwrap_or(0),
      Max => self.items.len() - 1,
      Min => 0,
    });
  }
}

enum SelectionChangeMode {
  Restore,
  Max,
  Min,
}
