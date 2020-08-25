#![allow(dead_code)]

use tui::layout::{Constraint, Direction, Layout};

pub struct NavFrame<'a, M> {
  root: Item<'a, M>,
  next: Vec<usize>,
}

impl<'a, M> NavFrame<'a, M> {
  pub fn start_frame(&mut self) {
    self.next.clear();
  }

  pub fn nav(&mut self, direction: Direction, constraints: impl Into<Vec<Constraint>>) {
    let next = self.next.clone();
    match self.get_item(&next) {
      Some(Item::Container(container)) => {
        container.modify(direction, constraints);
      }
      Some(Item::Widget(handler)) => {
        panic!("Navigation container cannot be nested into widgets");
      }
      None => {
        // TODO: have to add new item to parent. Need to get parent first
      }
    }
    self.next.push(0);
  }

  pub fn rows(&mut self, constraints: impl Into<Vec<Constraint>>) {
    self.nav(Direction::Vertical, constraints);
  }

  pub fn cols(&mut self, constraints: impl Into<Vec<Constraint>>) {
    self.nav(Direction::Horizontal, constraints);
  }

  pub fn widget(&mut self, handler: Box<dyn FnMut(M) + 'a>) {

  }


  fn get_item<'b>(&'a mut self, stack: &'b [usize]) -> (Option<&'a mut Item<'a, M>>, Option<&'a mut Item<'a, M>>) {
    let mut parent = None;
    let mut current = &mut self.root;
    for index in stack {
      match current {
        Item::Widget(_) => None,
        Item::Container(container) => {
          current = container.items.get_mut(*index)
        }
      }
    }
    Some(current)
  }
}

// Container

enum Item<'a, M> {
  Container(Container<'a, M>),
  Widget(Box<dyn FnMut(M) + 'a>),
}

struct Container<'a, M> {
  items: Vec<Item<'a, M>>,
  direction: Direction,
  layout: Layout,
  selection_index: Option<usize>,
  last_selection_index: Option<usize>,
}

// Creation & modification

impl<'a, M> Container<'a, M> {
  fn new(direction: Direction, constraints: impl Into<Vec<Constraint>>) -> Container<'a, M> {
    let constraints = constraints.into();
    let len = constraints.len();
    assert_ne!(len, 0);
    Self {
      items: Vec::with_capacity(constraints.len()),
      direction: direction.clone(),
      layout: Layout::default().direction(direction).constraints(constraints),
      selection_index: None,
      last_selection_index: None,
    }
  }

  fn rows(constraints: impl Into<Vec<Constraint>>) -> Self {
    Self::new(Direction::Vertical, constraints)
  }

  fn cols(constraints: impl Into<Vec<Constraint>>) -> Self {
    Self::new(Direction::Horizontal, constraints)
  }

  fn modify(&mut self, direction: Direction, constraints: impl Into<Vec<Constraint>>) {
    let constraints = constraints.into();
    let len = constraints.len();
    assert_ne!(len, 0);
    self.layout = Layout::default().direction(direction).constraints(constraints);
    if self.direction != direction {
      self.direction = direction;
      self.selection_index = None;
      self.last_selection_index = None;
    } else {
      self.selection_index = self.selection_index.map(|i| i.min(len - 1));
      self.last_selection_index = self.last_selection_index.map(|i| i.min(len - 1));
    }
  }
}

// Message handling

impl<'a, M> Container<'a, M> {
  pub fn message(&mut self, message: M) {
    for (index, item) in self.items.iter_mut().enumerate() {
      if self.selection_index.map_or(false, |i| index == i) {
        match item {
          Item::Widget(f) => {
            (f)(message);
            break;
          }
          Item::Container(container) => {
            container.message(message);
            break;
          }
        }
      }
    }
  }
}

// Selection

impl<'a, M> Container<'a, M> {
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
        if let Some(Item::Container(container)) = self.items.get_mut(selection_index) {
          container.unselect();
        }
        self.selection_index = Some(new_selection_index);
        self.last_selection_index = self.selection_index;
        let self_direction = self.direction.clone();
        if let Some(Item::Container(container)) = self.items.get_mut(new_selection_index) {
          let mode = Self::selection_change_mode(self_direction, direction.clone(), up_left);
          container.select(mode);
        }
      }
      true
    } else {
      for item in self.items.iter_mut() {
        if let Item::Container(container) = item {
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
