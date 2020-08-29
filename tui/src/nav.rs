#![allow(dead_code)]

use tui::layout::{Constraint, Direction, Layout};

pub struct NavFrame<'m, M> {
  root: Option<Item<'m, M>>,
  next: Vec<usize>,
}

impl<'n, 'm, M: 'm> NavFrame<'m, M> {
  pub fn start_frame(&mut self) {
    self.next.clear();
  }

  pub fn nav(&'n mut self, direction: Direction, constraints: impl Into<Vec<Constraint>>) {
    let next = self.next.clone();
    match self.get_item(next) {
      GetItem::CreateRoot => {
        self.root = Some(Item::Container(Container::new(direction, constraints)))
      }
      GetItem::CreateInParent(parent) => {
        parent.add_item(Item::Container(Container::new(direction, constraints)))
      }
      GetItem::Modify(parent, index, item) => {
        match item {
          Item::Container(container) => {
            container.modify(direction, constraints);
          }
          Item::Widget(_) => {
            parent.unwrap_or_else(|| panic!()).replace_item(index, Item::Container(Container::new(direction, constraints)));
          }
        }
      }
    }
    self.next.push(0);
  }

  // pub fn rows(&'n mut self, constraints: impl Into<Vec<Constraint>>) {
  //   self.nav(Direction::Vertical, constraints);
  // }
  //
  // pub fn cols(&'n mut self, constraints: impl Into<Vec<Constraint>>) {
  //   self.nav(Direction::Horizontal, constraints);
  // }
  //
  // pub fn widget(&'m mut self, _handler: Box<dyn FnMut(M) + 'm>) {
  //   // TODO
  // }


  fn get_item(&'n mut self, stack: Vec<usize>) -> GetItem<'n, 'm, M> {
    let mut parent = None;
    let mut current: Option<&'n mut Item<'m, M>> = self.root.as_mut();
    for (i, item_index) in stack.iter().enumerate() {
      let last = i == stack.len() - 1;
      match current {
        Some(Item::Widget(_)) => panic!("Attempted to get item with index '{}' from within a widget", item_index),
        Some(Item::Container(container)) => {
          parent = Some(container);
          current = container.items.get_mut(*item_index)
        }
        None if !last => panic!("Attempted to get item with index '{}', but the current item is None and it is not the last one", item_index),
        None => {}
      }
    }
    match (parent, current) {
      (None, None) => GetItem::<'n, 'm, M>::CreateRoot,
      (Some(container), None) => GetItem::<'n, 'm, M>::CreateInParent(container),
      (container, Some(item)) => GetItem::<'n, 'm, M>::Modify(container, *stack.last().unwrap(), item),
    }
  }
}

enum GetItem<'n, 'm, M: 'm> {
  CreateRoot,
  CreateInParent(&'n mut Container<'m, M>),
  Modify(Option<&'n mut Container<'m, M>>, usize, &'n mut Item<'m, M>),
}

// Container

enum Item<'m, M: 'm> {
  Container(Container<'m, M>),
  Widget(Box<dyn FnMut(M) + 'm>),
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
    self.layout = Layout::default().direction(direction.clone()).constraints(constraints);
    if self.direction != direction {
      self.direction = direction;
      self.selection_index = None;
      self.last_selection_index = None;
    } else {
      self.selection_index = self.selection_index.map(|i| i.min(len - 1));
      self.last_selection_index = self.last_selection_index.map(|i| i.min(len - 1));
    }
  }

  fn add_item(&mut self, item: Item<'a, M>) {
    self.items.push(item);
  }

  fn replace_item(&mut self, index: usize, item: Item<'a, M>) {
    *self.items.get_mut(index)
      .unwrap_or_else(|| panic!("Cannot replace item at index {}, there is no item for that index", index)) = item;
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
