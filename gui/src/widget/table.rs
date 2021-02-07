use std::hash::{Hash, Hasher as StdHasher};

use iced::Vector;
use iced_graphics::{Backend, Defaults, Primitive, Renderer};
use iced_native::{Align, Background, Clipboard, Color, Container, Element, event, Event, Hasher, layout, Layout, layout::flex, Length, mouse, overlay, Point, Rectangle, Row, Size, Widget};
use iced_native::event::Status;
use iced_native::layout::{Limits, Node};
use tracing::trace;

pub struct Table<'a, M, R> {
  // Properties for the entire table.
  width: Length,
  height: Length,
  max_width: u32,
  max_height: u32,
  // Properties for elements inside the table.
  spacing: u32,
  columns: Vec<TableColumn<'a, M, R>>,
  row_height: u32,
  rows: Vec<Vec<Element<'a, M, R>>>,
}

pub struct TableColumn<'a, M, R> {
  width_fill_portion: u32,
  header: Element<'a, M, R>,
}

impl<'a, M, R> Table<'a, M, R> {
  pub fn new() -> Self {
    Self {
      width: Length::Fill,
      height: Length::Fill,
      max_width: u32::MAX,
      max_height: u32::MAX,

      spacing: 0,
      columns: Vec::new(),
      row_height: 16,
      rows: Vec::new(),
    }
  }


  pub fn width(mut self, width: Length) -> Self {
    self.width = width;
    self
  }

  pub fn height(mut self, height: Length) -> Self {
    self.height = height;
    self
  }

  pub fn max_width(mut self, max_width: u32) -> Self {
    self.max_width = max_width;
    self
  }

  pub fn max_height(mut self, max_height: u32) -> Self {
    self.max_height = max_height;
    self
  }


  pub fn spacing(mut self, spacing: u32) -> Self {
    self.spacing = spacing;
    self
  }

  pub fn push_column<E>(mut self, width_fill_portion: u32, header: E) -> Self
    where E: Into<Element<'a, M, R>>
  {
    let header = header.into();
    self.columns.push(TableColumn { width_fill_portion, header });
    self
  }

  pub fn row_height(mut self, height: u32) -> Self {
    self.row_height = height;
    self
  }

  pub fn push_row(mut self, row: Vec<Element<'a, M, R>>) -> Self {
    self.rows.push(row);
    self
  }
}

impl<'a, M, B> Widget<M, Renderer<B>> for Table<'a, M, Renderer<B>>
  where B: Backend
{
  fn width(&self) -> Length { self.width }

  fn height(&self) -> Length { self.height }

  fn layout(&self, renderer: &Renderer<B>, limits: &Limits) -> Node {
    let limits = limits
      .max_width(self.max_width)
      .max_height(self.max_height)
      .width(self.width)
      .height(self.height);
    let fill = limits.fill();
    let width = fill.width;
    let spacing = self.spacing as usize;
    let row_height_and_spacing = self.row_height as usize + spacing;
    let height = row_height_and_spacing + (self.rows.len() * row_height_and_spacing) - spacing;
    Node::new(Size::new(width, height as f32))
  }

  fn draw(
    &self,
    renderer: &mut Renderer<B>,
    defaults: &Defaults,
    layout: Layout<'_>,
    cursor_position: Point,
    viewport: &Rectangle<f32>,
  ) -> (Primitive, mouse::Interaction) {
    let absolute_position = layout.position();
    let offset = Vector::new(absolute_position.x, absolute_position.y);
    let row_height = self.row_height as f32;
    let spacing = self.spacing as f32;
    let total_width = layout.bounds().width;

    struct ColumnLayout {
      width: f32,
      x_offset: f32,
    }
    let column_layouts = {
      let num_columns = self.columns.len();
      let last_column_index = (num_columns - 1).max(0);
      let num_spacers = num_columns.saturating_sub(1);
      let total_spacing = (self.spacing as usize * num_spacers) as f32;
      let total_space = total_width - total_spacing;
      let total_fill_portion = self.columns.iter().map(|c| c.width_fill_portion).sum::<u32>() as f32;
      let mut layouts = Vec::new();
      let mut x_offset = 0f32;
      for (i, column) in self.columns.iter().enumerate() {
        let width = (column.width_fill_portion as f32 / total_fill_portion) * total_space;
        layouts.push(ColumnLayout { width, x_offset });
        x_offset += width;
        if i < last_column_index {
          x_offset += spacing;
        }
      }
      layouts
    };

    let mut mouse_cursor = mouse::Interaction::default();
    let mut primitives = Vec::new();
    if viewport.x <= row_height {
      for (column, column_layout) in self.columns.iter().zip(&column_layouts) {
        let mut node = Node::new(Size::new(column_layout.width, row_height));
        node.move_to(Point::new(column_layout.x_offset, 0f32));
        let layout = Layout::with_offset(offset, &node);
        let (primitive, new_mouse_cursor) = column.header.draw(renderer, defaults, layout, cursor_position, viewport);
        if new_mouse_cursor > mouse_cursor { mouse_cursor = new_mouse_cursor; }
        primitives.push(primitive);
      }
    }
    if !self.rows.is_empty() {
      let num_rows = self.rows.len();
      let last_row_index = (num_rows - 1).max(0);
      let row_height_plus_spacing = row_height + spacing;
      let y = (viewport.y - row_height_plus_spacing).max(0f32);
      let start_offset = ((y / row_height_plus_spacing).floor() as usize).min(last_row_index);
      let height = (viewport.height - row_height_plus_spacing).max(0f32);
      // TODO: figure out why this + 1 is needed. I added it because the last row did not always seem visible from a certain y offset. May be a float precision issue?
      let num_rows_to_render = (height / row_height_plus_spacing).ceil() as usize + 1;
      let end_offset = (start_offset + num_rows_to_render).min(last_row_index);

      let mut y_offset = row_height_plus_spacing + (start_offset as f32 * row_height_plus_spacing);
      for i in start_offset..=end_offset {
        let row = &self.rows[i]; // OPTO: get_unchecked
        for (cell, column_layout) in row.iter().zip(&column_layouts) {
          let mut node = Node::new(Size::new(column_layout.width, row_height));
          node.move_to(Point::new(column_layout.x_offset, y_offset));
          let layout = Layout::with_offset(offset, &node);
          let (primitive, new_mouse_cursor) = cell.draw(renderer, defaults, layout, cursor_position, viewport);
          if new_mouse_cursor > mouse_cursor { mouse_cursor = new_mouse_cursor; }
          primitives.push(primitive);
        }
        y_offset += row_height;
        if i < last_row_index {
          y_offset += spacing;
        }
      }
    }
    (Primitive::Group { primitives }, mouse_cursor)
  }

  fn hash_layout(&self, state: &mut Hasher) {
    struct Marker;
    std::any::TypeId::of::<Marker>().hash(state);
    self.width.hash(state);
    self.height.hash(state);
    self.max_width.hash(state);
    self.max_height.hash(state);
    self.spacing.hash(state);
    for column in &self.columns {
      state.write_u32(column.width_fill_portion);
    }
    self.row_height.hash(state);
    state.write_usize(self.rows.len());
  }

  fn on_event(
    &mut self,
    event: Event,
    layout: Layout<'_>,
    cursor_position: Point,
    messages: &mut Vec<M>,
    renderer: &Renderer<B>,
    clipboard: Option<&dyn Clipboard>,
  ) -> Status {
    event::Status::Ignored
  }

  fn overlay(
    &mut self,
    layout: Layout<'_>,
  ) -> Option<overlay::Element<'_, M, Renderer<B>>> {
    None
  }
}

impl<'a, M, B> Into<Element<'a, M, Renderer<B>>> for Table<'a, M, Renderer<B>>
  where
    M: 'a,
    B: 'a + Backend,
{
  fn into(self) -> Element<'a, M, Renderer<B>> {
    Element::new(self)
  }
}
