use std::hash::Hash;

use iced::Vector;
use iced_graphics::{Backend, Primitive, Renderer as ConcreteRenderer};
use iced_native::{
  Align, Background, Clipboard, Color, Container, Element, event, Event, Hasher, layout, Layout, layout::flex, Length,
  mouse, overlay, Point, Rectangle, Renderer, Row, Scrollable, scrollable, Size, Widget,
};
use iced_native::event::Status;
use iced_native::layout::{Limits, Node};
use tracing::trace;

//
// Table builder
//

pub struct TableBuilder<'a, M, R> {
  width: Length,
  height: Length,
  max_width: u32,
  max_height: u32,
  padding: u32,
  spacing: u32,
  header: TableHeader<'a, M, R>,
  rows: TableRows<'a, M, R>,
}

impl<'a, M, R> TableBuilder<'a, M, R> {
  pub fn new() -> Self {
    let spacing = 0;
    let row_height = 16;
    Self {
      width: Length::Fill,
      height: Length::Fill,
      max_width: u32::MAX,
      max_height: u32::MAX,
      padding: 0,
      spacing: 0,
      header: TableHeader { spacing, row_height, column_fill_portions: Vec::new(), headers: Vec::new() },
      rows: TableRows { spacing, row_height, column_fill_portions: Vec::new(), rows: Vec::new() },
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

  pub fn padding(mut self, padding: u32) -> Self {
    self.padding = padding;
    self
  }


  pub fn spacing(mut self, spacing: u32) -> Self {
    self.spacing = spacing;
    self.header.spacing = spacing;
    self.rows.spacing = spacing;
    self
  }

  pub fn push_column<E>(mut self, width_fill_portion: u32, header: E) -> Self
    where E: Into<Element<'a, M, R>>
  {
    self.header.column_fill_portions.push(width_fill_portion);
    self.header.headers.push(header.into());
    self.rows.column_fill_portions.push(width_fill_portion);
    self
  }

  pub fn header_row_height(mut self, height: u32) -> Self {
    self.header.row_height = height;
    self
  }

  pub fn push_row(mut self, row: Vec<Element<'a, M, R>>) -> Self {
    self.rows.rows.push(row);
    self
  }

  pub fn row_height(mut self, height: u32) -> Self {
    self.rows.row_height = height;
    self
  }


  pub fn build(self, rows_scrollable_state: &'a mut scrollable::State) -> Table<'a, M, R> where M: 'a, R: 'a + TableRenderer {
    let rows = Scrollable::new(rows_scrollable_state).push(Element::new(self.rows));
    Table {
      width: self.width,
      height: self.height,
      max_width: self.max_width,
      max_height: self.max_height,
      padding: self.padding,
      spacing: self.spacing,
      header: self.header,
      rows,
    }
  }
}

//
// Table widget
//

pub struct Table<'a, M, R: TableRenderer> {
  width: Length,
  height: Length,
  max_width: u32,
  max_height: u32,
  padding: u32,
  spacing: u32,
  header: TableHeader<'a, M, R>,
  rows: Scrollable<'a, M, R>,
}

impl<'a, M, R: TableRenderer> Widget<M, R> for Table<'a, M, R> {
  fn width(&self) -> Length { self.width }

  fn height(&self) -> Length { self.height }

  fn layout(&self, renderer: &R, limits: &Limits) -> Node {
    let padding = self.padding as f32;
    let spacing = self.spacing as f32;

    let limits = limits
      .max_width(self.max_width)
      .max_height(self.max_height)
      .width(self.width)
      .height(self.height)
      .pad(padding);
    let header_layout = self.header.layout(renderer, &limits);
    let header_size = header_layout.size();
    let header_y_offset = header_size.height + spacing;

    let limits = limits.shrink(Size::new(0f32, header_y_offset));
    let mut rows_layout = self.rows.layout(renderer, &limits);
    rows_layout.move_to(Point::new(0f32, header_y_offset));
    let rows_size = rows_layout.size();

    let size = Size::new(rows_size.width.max(rows_size.width), header_size.height + spacing + rows_size.height);
    let mut layout = Node::with_children(size, vec![header_layout, rows_layout]);
    layout.move_to(Point::new(padding, padding));
    layout
  }

  fn draw(
    &self,
    renderer: &mut R,
    defaults: &R::Defaults,
    layout: Layout<'_>,
    cursor_position: Point,
    viewport: &Rectangle<f32>,
  ) -> R::Output {
    renderer.draw_table(defaults, layout, cursor_position, viewport, &self.header, &self.rows)
  }

  fn hash_layout(&self, state: &mut Hasher) {
    struct Marker;
    std::any::TypeId::of::<Marker>().hash(state);
    self.width.hash(state);
    self.height.hash(state);
    self.max_width.hash(state);
    self.max_height.hash(state);
    self.padding.hash(state);
    self.spacing.hash(state);
    self.header.hash_layout(state);
    self.rows.hash_layout(state);
  }

  fn on_event(
    &mut self,
    event: Event,
    layout: Layout<'_>,
    cursor_position: Point,
    messages: &mut Vec<M>,
    renderer: &R,
    clipboard: Option<&dyn Clipboard>,
  ) -> Status {
    let mut layout_iter = layout.children();
    let header_status = self.header.on_event(event.clone(), layout_iter.next().unwrap(), cursor_position, messages, renderer, clipboard);
    if header_status == Status::Captured { return Status::Captured; }
    self.rows.on_event(event, layout_iter.next().unwrap(), cursor_position, messages, renderer, clipboard)
  }

  fn overlay(&mut self, layout: Layout<'_>) -> Option<overlay::Element<'_, M, R>> {
    let mut layout_iter = layout.children();
    let header_overlay = self.header.overlay(layout_iter.next().unwrap());
    if header_overlay.is_some() { return header_overlay; }
    self.rows.overlay(layout_iter.next().unwrap())
  }
}

pub trait TableRenderer: TableHeaderRenderer + scrollable::Renderer + TableRowsRenderer {
  fn draw_table<M>(
    &mut self,
    defaults: &Self::Defaults,
    layout: Layout<'_>,
    cursor_position: Point,
    viewport: &Rectangle<f32>,
    header: &TableHeader<'_, M, Self>,
    rows: &Scrollable<'_, M, Self>,
  ) -> Self::Output;
}

impl<B: Backend> TableRenderer for ConcreteRenderer<B> {
  fn draw_table<M>(
    &mut self,
    defaults: &Self::Defaults,
    layout: Layout<'_>,
    cursor_position: Point,
    viewport: &Rectangle<f32>,
    header: &TableHeader<'_, M, Self>,
    rows: &Scrollable<'_, M, Self>,
  ) -> Self::Output {
    let mut layout_iter = layout.children();
    let mut mouse_cursor = mouse::Interaction::default();
    let mut primitives = Vec::new();
    let (primitive, new_mouse_cursor) = header.draw(self, defaults, layout_iter.next().unwrap(), cursor_position, viewport);
    if new_mouse_cursor > mouse_cursor { mouse_cursor = new_mouse_cursor; }
    primitives.push(primitive);
    let (primitive, new_mouse_cursor) = rows.draw(self, defaults, layout_iter.next().unwrap(), cursor_position, viewport);
    if new_mouse_cursor > mouse_cursor { mouse_cursor = new_mouse_cursor; }
    primitives.push(primitive);
    (Primitive::Group { primitives }, mouse_cursor)
  }
}

impl<'a, M: 'a, R: 'a + TableRenderer> Into<Element<'a, M, R>> for Table<'a, M, R> {
  fn into(self) -> Element<'a, M, R> {
    Element::new(self)
  }
}

//
// Table header
//

pub struct TableHeader<'a, M, R> {
  spacing: u32,
  row_height: u32,
  column_fill_portions: Vec<u32>,
  headers: Vec<Element<'a, M, R>>,
}

impl<'a, M, R: TableHeaderRenderer> Widget<M, R> for TableHeader<'a, M, R> {
  fn width(&self) -> Length { Length::Fill }
  fn height(&self) -> Length { Length::Fill }

  fn layout(&self, renderer: &R, limits: &Limits) -> Node {
    let total_width = limits.max().width;
    let total_height = self.row_height as f32;
    let layouts = layout_columns(total_width, total_height, self.column_fill_portions.iter().copied(), self.spacing);
    Node::with_children(Size::new(total_width, total_height), layouts)
  }

  fn draw(
    &self,
    renderer: &mut R,
    defaults: &R::Defaults,
    layout: Layout<'_>,
    cursor_position: Point,
    viewport: &Rectangle<f32>,
  ) -> R::Output {
    renderer.draw_table_header(defaults, layout, cursor_position, viewport, self.row_height as f32, &self.headers)
  }

  fn hash_layout(&self, state: &mut Hasher) {
    struct Marker;
    std::any::TypeId::of::<Marker>().hash(state);
    self.spacing.hash(state);
    self.row_height.hash(state);
    for column_fill_portion in &self.column_fill_portions {
      column_fill_portion.hash(state);
    }
  }
}

pub trait TableHeaderRenderer: Renderer {
  fn draw_table_header<M>(
    &mut self,
    defaults: &Self::Defaults,
    layout: Layout<'_>,
    cursor_position: Point,
    viewport: &Rectangle<f32>,
    row_height: f32,
    headers: &[Element<'_, M, Self>],
  ) -> Self::Output;
}

impl<B: Backend> TableHeaderRenderer for ConcreteRenderer<B> {
  fn draw_table_header<M>(
    &mut self,
    defaults: &Self::Defaults,
    layout: Layout<'_>,
    cursor_position: Point,
    viewport: &Rectangle<f32>,
    row_height: f32,
    headers: &[Element<'_, M, Self>],
  ) -> Self::Output {
    let mut mouse_cursor = mouse::Interaction::default();
    if headers.is_empty() {
      return (Primitive::None, mouse_cursor);
    }
    let mut primitives = Vec::new();
    for (header, layout) in headers.iter().zip(layout.children()) {
      let (primitive, new_mouse_cursor) = header.draw(self, defaults, layout, cursor_position, viewport);
      if new_mouse_cursor > mouse_cursor { mouse_cursor = new_mouse_cursor; }
      primitives.push(primitive);
    }
    (Primitive::Group { primitives }, mouse_cursor)
  }
}

impl<'a, M: 'a, R: 'a + TableHeaderRenderer> Into<Element<'a, M, R>> for TableHeader<'a, M, R> {
  fn into(self) -> Element<'a, M, R> {
    Element::new(self)
  }
}

//
// Table rows
//

struct TableRows<'a, M, R> {
  spacing: u32,
  row_height: u32,
  column_fill_portions: Vec<u32>,
  rows: Vec<Vec<Element<'a, M, R>>>,
}

impl<'a, M, R: TableRowsRenderer> Widget<M, R> for TableRows<'a, M, R> {
  fn width(&self) -> Length { Length::Fill }
  fn height(&self) -> Length { Length::Fill }

  fn layout(&self, renderer: &R, limits: &Limits) -> Node {
    let max = limits.max();
    let total_width = max.width;
    let layouts = layout_columns(total_width, self.row_height as f32, self.column_fill_portions.iter().copied(), self.spacing);
    let num_rows = self.rows.len();
    let total_height = num_rows * self.row_height as usize + num_rows.saturating_sub(1) * self.spacing as usize;
    Node::with_children(Size::new(total_width, total_height as f32), layouts)
  }

  fn draw(
    &self,
    renderer: &mut R,
    defaults: &R::Defaults,
    layout: Layout<'_>,
    cursor_position: Point,
    viewport: &Rectangle<f32>,
  ) -> R::Output {
    renderer.draw_table_rows(defaults, layout, cursor_position, viewport, self.row_height as f32, self.spacing as f32, &self.rows)
  }

  fn hash_layout(&self, state: &mut Hasher) {
    struct Marker;
    std::any::TypeId::of::<Marker>().hash(state);
    self.spacing.hash(state);
    self.row_height.hash(state);
    self.rows.len().hash(state);
  }
}

pub trait TableRowsRenderer: Renderer {
  fn draw_table_rows<M>(
    &mut self,
    defaults: &Self::Defaults,
    layout: Layout<'_>,
    cursor_position: Point,
    viewport: &Rectangle<f32>,
    row_height: f32,
    spacing: f32,
    rows: &Vec<Vec<Element<'_, M, Self>>>,
  ) -> Self::Output;
}

impl<B: Backend> TableRowsRenderer for ConcreteRenderer<B> {
  fn draw_table_rows<M>(
    &mut self,
    defaults: &Self::Defaults,
    layout: Layout<'_>,
    cursor_position: Point,
    viewport: &Rectangle<f32>,
    row_height: f32,
    spacing: f32,
    rows: &Vec<Vec<Element<'_, M, Self>>>,
  ) -> Self::Output {
    let absolute_position = layout.position();
    let offset = Vector::new(absolute_position.x, absolute_position.y);

    let mut mouse_cursor = mouse::Interaction::default();
    if rows.is_empty() {
      return (Primitive::None, mouse_cursor);
    }
    let mut primitives = Vec::new();

    let num_rows = rows.len();
    let last_row_index = num_rows.saturating_sub(1);
    let row_height_plus_spacing = row_height + spacing;
    let relative_y = viewport.y - absolute_position.y;
    let start_offset = ((relative_y / row_height_plus_spacing).floor() as usize).min(last_row_index);
    let num_rows_to_render = (viewport.height / row_height_plus_spacing).ceil() as usize;
    let end_offset = (start_offset + num_rows_to_render).min(last_row_index);

    let mut y_offset = start_offset as f32 * row_height_plus_spacing;
    for i in start_offset..=end_offset {
      let row = &rows[i]; // OPTO: get_unchecked
      for (cell, base_layout) in row.iter().zip(layout.children()) {
        // Reconstruct the layout from `base_layout` which has a correct x position, but an incorrect y position which
        // always points to the first row. This is needed so that we do not have to lay out all the cells of the table
        // each time the layout changes. Now we only calculate the absolute layout of cells which are in view.
        let bounds = base_layout.bounds();
        let mut node = Node::new(Size::new(bounds.width, bounds.height));
        node.move_to(Point::new(bounds.x, y_offset));
        let layout = Layout::with_offset(offset, &node);

        let (primitive, new_mouse_cursor) = cell.draw(self, defaults, layout, cursor_position, viewport);
        if new_mouse_cursor > mouse_cursor { mouse_cursor = new_mouse_cursor; }
        primitives.push(primitive);
      }
      y_offset += row_height;
      if i < last_row_index {
        y_offset += spacing;
      }
    }

    (Primitive::Group { primitives }, mouse_cursor)
  }
}

impl<'a, M: 'a, R: 'a + TableRowsRenderer> Into<Element<'a, M, R>> for TableRows<'a, M, R> {
  fn into(self) -> Element<'a, M, R> {
    Element::new(self)
  }
}

//
// Column layout calculation
//

fn layout_columns(total_width: f32, row_height: f32, width_fill_portions: impl Iterator<Item=u32> + Clone, spacing: u32) -> Vec<Node> {
  let num_columns = width_fill_portions.clone().count();
  let last_column_index = (num_columns - 1).max(0);
  let num_spacers = num_columns.saturating_sub(1);
  let total_spacing = (spacing as usize * num_spacers) as f32;
  let total_space = total_width - total_spacing;
  let total_fill_portion = width_fill_portions.clone().sum::<u32>() as f32;
  let mut layouts = Vec::new();
  let mut x_offset = 0f32;
  for (i, width_fill_portion) in width_fill_portions.enumerate() {
    let width = (width_fill_portion as f32 / total_fill_portion) * total_space;
    let mut layout = Node::new(Size::new(width, row_height));
    layout.move_to(Point::new(x_offset, 0f32));
    layouts.push(layout);
    x_offset += width;
    if i < last_column_index {
      x_offset += spacing as f32;
    }
  }
  layouts
}
