#![allow(dead_code)]

use std::hash::Hash;

use iced_graphics::{Backend, Primitive, Renderer as ConcreteRenderer};
use iced_native::{
  Clipboard, Element, Event, event, Hasher, Layout, Length, mouse, overlay, Point, Rectangle, Renderer, Scrollable,
  scrollable, Size, touch, Widget,
};
use iced_native::event::Status;
use iced_native::layout::{Limits, Node};

//
// Table builder
//

// OPTO: Instead of rendering rows, render columns. Right now the mapper functions are dynamic dispatch because they
//       have different types, and we call the mapper on each cell. If we render columns instead, we only need one
//       dynamic dispatch per column. We can then also turn `&T` into `T` on the mapper. We do have to iterate over the
//       rows multiple times though, but this is possible because it is `Clone`. It might be a little bit slow because
//       `skip` on `Iterator` could be slow.

pub struct TableBuilder<'a, T, I, M, R> where
  T: 'a,
  I: 'a + Iterator<Item=T> + ExactSizeIterator + Clone,
{
  width: Length,
  height: Length,
  max_width: u32,
  max_height: u32,
  spacing: u32,
  header: TableHeader<'a, M, R>,
  rows: TableRows<'a, T, I, M, R>,
}

impl<'a, T, I, M, R> TableBuilder<'a, T, I, M, R> where
  T: 'a,
  I: 'a + Iterator<Item=T> + ExactSizeIterator + Clone,
{
  pub fn new(rows: I) -> Self {
    let spacing = 0;
    let row_height = 16;
    Self {
      width: Length::Fill,
      height: Length::Fill,
      max_width: u32::MAX,
      max_height: u32::MAX,
      spacing: 0,
      header: TableHeader { spacing, row_height, column_fill_portions: Vec::new(), headers: Vec::new() },
      rows: TableRows { spacing, row_height, column_fill_portions: Vec::new(), mappers: Vec::new(), rows },
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
    self.header.spacing = spacing;
    self.rows.spacing = spacing;
    self
  }

  pub fn header_row_height(mut self, height: u32) -> Self {
    self.header.row_height = height;
    self
  }

  pub fn row_height(mut self, height: u32) -> Self {
    self.rows.row_height = height;
    self
  }


  pub fn push_column<E>(mut self, width_fill_portion: u32, header: E, mapper: Box<dyn 'a + Fn(&T) -> Element<'a, M, R>>) -> Self where
    E: Into<Element<'a, M, R>>
  {
    self.header.column_fill_portions.push(width_fill_portion);
    self.header.headers.push(header.into());
    self.rows.column_fill_portions.push(width_fill_portion);
    self.rows.mappers.push(mapper);
    self
  }


  pub fn build(
    self,
    rows_scrollable_state: &'a mut scrollable::State,
  ) -> Table<'a, M, R> where
    M: 'a,
    R: 'a + TableRenderer + TableRowsRenderer<'a, T, I, M>
  {
    let rows = Scrollable::new(rows_scrollable_state).push(Element::new(self.rows));
    Table {
      width: self.width,
      height: self.height,
      max_width: self.max_width,
      max_height: self.max_height,
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
  spacing: u32,
  header: TableHeader<'a, M, R>,
  rows: Scrollable<'a, M, R>,
}

impl<'a, M, R: TableRenderer> Widget<M, R> for Table<'a, M, R> {
  fn width(&self) -> Length { self.width }

  fn height(&self) -> Length { self.height }

  fn layout(&self, renderer: &R, limits: &Limits) -> Node {
    let spacing = self.spacing as f32;
    let limits = limits
      .max_width(self.max_width)
      .max_height(self.max_height)
      .width(self.width)
      .height(self.height);

    let header_layout = self.header.layout(renderer, &limits);
    let header_size = header_layout.size();
    let header_y_offset = header_size.height + spacing;

    let limits = limits.shrink(Size::new(0f32, header_y_offset));
    let mut rows_layout = self.rows.layout(renderer, &limits);
    rows_layout.move_to(Point::new(0f32, header_y_offset));
    let rows_size = rows_layout.size();

    let size = Size::new(rows_size.width.max(rows_size.width), header_size.height + spacing + rows_size.height);
    Node::with_children(size, vec![header_layout, rows_layout])
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
    if let (Some(header_layout), Some(rows_layout)) = unfold_table_layout(layout) {
      let header_status = self.header.on_event(event.clone(), header_layout, cursor_position, messages, renderer, clipboard);
      if header_status == Status::Captured { return Status::Captured; }
      self.rows.on_event(event, rows_layout, cursor_position, messages, renderer, clipboard)
    } else {
      Status::Ignored
    }
  }

  fn overlay(&mut self, layout: Layout<'_>) -> Option<overlay::Element<'_, M, R>> {
    if let (Some(header_layout), Some(rows_layout)) = unfold_table_layout(layout) {
      let header_overlay = self.header.overlay(header_layout);
      if header_overlay.is_some() { return header_overlay; }
      self.rows.overlay(rows_layout)
    } else {
      None
    }
  }
}

fn unfold_table_layout(layout: Layout<'_>) -> (Option<Layout<'_>>, Option<Layout<'_>>) {
  let mut layout_iter = layout.children();
  (layout_iter.next(), layout_iter.next())
}

pub trait TableRenderer: TableHeaderRenderer + scrollable::Renderer {
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
    let mut mouse_cursor = mouse::Interaction::default();
    let mut primitives = Vec::new();
    if let (Some(header_layout), Some(rows_layout)) = unfold_table_layout(layout) {
      let (primitive, new_mouse_cursor) = header.draw(self, defaults, header_layout, cursor_position, viewport);
      if new_mouse_cursor > mouse_cursor { mouse_cursor = new_mouse_cursor; }
      primitives.push(primitive);
      let (primitive, new_mouse_cursor) = rows.draw(self, defaults, rows_layout, cursor_position, viewport);
      if new_mouse_cursor > mouse_cursor { mouse_cursor = new_mouse_cursor; }
      primitives.push(primitive);
    }
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

  fn layout(&self, _renderer: &R, limits: &Limits) -> Node {
    let total_width = limits.max().width;
    let total_height = self.row_height as f32;
    let layouts = layout_columns(total_width, total_height, &self.column_fill_portions, self.spacing);
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
    renderer.draw_table_header(defaults, layout, cursor_position, viewport, &self.headers)
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

  fn on_event(
    &mut self,
    event: Event,
    layout: Layout<'_>,
    cursor_position: Point,
    messages: &mut Vec<M>,
    renderer: &R,
    clipboard: Option<&dyn Clipboard>,
  ) -> Status {
    self.headers
      .iter_mut()
      .zip(layout.children())
      .map(|(header, layout)| {
        header.on_event(
          event.clone(),
          layout,
          cursor_position,
          messages,
          renderer,
          clipboard,
        )
      })
      .fold(event::Status::Ignored, event::Status::merge)
  }

  fn overlay(&mut self, layout: Layout<'_>) -> Option<overlay::Element<'_, M, R>> {
    self.headers
      .iter_mut()
      .zip(layout.children())
      .filter_map(|(header, layout)| header.overlay(layout))
      .next()
  }
}

pub trait TableHeaderRenderer: Renderer {
  fn draw_table_header<M>(
    &mut self,
    defaults: &Self::Defaults,
    layout: Layout<'_>,
    cursor_position: Point,
    viewport: &Rectangle<f32>,
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

struct TableRows<'a, T, I, M, R> where
  T: 'a,
  I: 'a + Iterator<Item=T> + ExactSizeIterator + Clone,
{
  spacing: u32,
  row_height: u32,
  column_fill_portions: Vec<u32>,
  mappers: Vec<Box<dyn 'a + Fn(&T) -> Element<'a, M, R>>>,
  rows: I,
}

impl<'a, T, I, M, R: TableRowsRenderer<'a, T, I, M>> Widget<M, R> for TableRows<'a, T, I, M, R> where
  T: 'a,
  I: 'a + Iterator<Item=T> + ExactSizeIterator + Clone,
{
  fn width(&self) -> Length { Length::Fill }
  fn height(&self) -> Length { Length::Fill }

  fn layout(&self, _renderer: &R, limits: &Limits) -> Node {
    let max = limits.max();
    let total_width = max.width;
    // HACK: only lay out first row, because laying out the entire table becomes slow for larger tables. Lay out the
    //       *visible rows* in the draw function.
    let layouts = layout_columns(total_width, self.row_height as f32, &self.column_fill_portions, self.spacing);
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
    renderer.draw_table_rows(defaults, layout, cursor_position, viewport, self.row_height as f32, self.spacing as f32, &self.mappers, self.rows.clone())
  }

  fn hash_layout(&self, state: &mut Hasher) {
    struct Marker;
    std::any::TypeId::of::<Marker>().hash(state);
    self.spacing.hash(state);
    self.row_height.hash(state);
    for column_fill_portion in &self.column_fill_portions {
      column_fill_portion.hash(state);
    }
    self.rows.len().hash(state);
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
    let absolute_position = layout.position();
    match &event {
      Event::Keyboard(_) | Event::Window(_) => return Status::Ignored,
      Event::Mouse(_) => {
        let mouse_position_relative = Point::new(cursor_position.x - absolute_position.x, cursor_position.y - absolute_position.y);
        if let Some(mut element) = self.get_element_at(mouse_position_relative, &layout) {
          // TODO: calculate new layout
          if element.on_event(event.clone(), layout, cursor_position, messages, renderer, clipboard) == Status::Captured {
            return Status::Captured;
          }
        }
      }
      Event::Touch(touch_event) => {
        let touch_position_absolute = match touch_event {
          touch::Event::FingerPressed { position, .. } => position,
          touch::Event::FingerMoved { position, .. } => position,
          touch::Event::FingerLifted { position, .. } => position,
          touch::Event::FingerLost { position, .. } => position,
        };
        let touch_position_relative = Point::new(touch_position_absolute.x - absolute_position.x, touch_position_absolute.y - absolute_position.y);
        // TODO: reduce code duplication
        if let Some(mut element) = self.get_element_at(touch_position_relative, &layout) {
          // TODO: calculate new layout
          if element.on_event(event.clone(), layout, cursor_position, messages, renderer, clipboard) == Status::Captured {
            return Status::Captured;
          }
        }
      }
    }
    Status::Ignored
  }
}

impl<'a, T, I, M, R: TableRowsRenderer<'a, T, I, M>> TableRows<'a, T, I, M, R> where
  T: 'a,
  I: 'a + Iterator<Item=T> + ExactSizeIterator + Clone,
{
  fn get_row_index_at(&self, y: f32) -> Option<usize> {
    if y < 0f32 { return None; } // Out of bounds
    let spacing = self.spacing as f32;
    let row_height = self.row_height as f32;
    let row_height_plus_spacing = row_height + spacing;
    let row_index = (y / row_height_plus_spacing).ceil() as usize;
    let row_offset_without_spacing = (row_index as f32 * row_height_plus_spacing) - spacing;
    if y > row_offset_without_spacing {
      None // On row spacing
    } else {
      Some(row_index.saturating_sub(1))
    }
  }

  fn get_column_index_at(&self, x: f32, layout: &Layout<'_>) -> Option<usize> {
    let spacing = self.spacing as f32;
    let mut offset = 0f32;
    for (column_index, column_layout) in layout.children().enumerate() {
      if x < offset { return None; } // On column spacing or out of bounds
      offset += column_layout.bounds().width;
      if x <= offset { return Some(column_index); }
      offset += spacing;
    }
    None
  }

  fn get_element_at(&self, point: Point, layout: &Layout<'_>) -> Option<Element<'a, M, R>> {
    let column_index = self.get_column_index_at(point.x, &layout);
    let mapper = column_index.and_then(|i| self.mappers.get(i));
    let row_index = self.get_row_index_at(point.y);
    let row = row_index.and_then(|i| self.rows.clone().nth(i));
    if let (Some(mapper), Some(row)) = (mapper, row) {
      Some(mapper(&row))
    } else {
      None
    }
  }
}

pub trait TableRowsRenderer<'a, T, I, M>: Renderer where
  T: 'a,
  I: 'a + Iterator<Item=T> + ExactSizeIterator,
{
  fn draw_table_rows(
    &mut self,
    defaults: &Self::Defaults,
    layout: Layout<'_>,
    cursor_position: Point,
    viewport: &Rectangle<f32>,
    row_height: f32,
    spacing: f32,
    mappers: &[Box<dyn 'a + Fn(&T) -> Element<'a, M, Self>>],
    rows: I,
  ) -> Self::Output;
}

impl<'a, T, I, M, B> TableRowsRenderer<'a, T, I, M> for ConcreteRenderer<B> where
  T: 'a,
  I: 'a + Iterator<Item=T> + ExactSizeIterator,
  B: Backend,
{
  fn draw_table_rows(
    &mut self,
    defaults: &Self::Defaults,
    layout: Layout<'_>,
    cursor_position: Point,
    viewport: &Rectangle<f32>,
    row_height: f32,
    spacing: f32,
    mappers: &[Box<dyn 'a + Fn(&T) -> Element<'a, M, Self>>],
    rows: I,
  ) -> Self::Output {
    let absolute_position = layout.position();
    let mut mouse_cursor = mouse::Interaction::default();
    let num_rows = rows.len();
    if num_rows == 0 {
      return (Primitive::None, mouse_cursor);
    }
    let mut primitives = Vec::new();
    let last_row_index = num_rows.saturating_sub(1);
    let row_height_plus_spacing = row_height + spacing;
    let start_offset = (((viewport.y - absolute_position.y) / row_height_plus_spacing).floor() as usize).min(last_row_index);
    // HACK: + 1 on next line to ensure that last partially visible row is not culled.
    let num_rows_to_render = ((viewport.height / row_height_plus_spacing).ceil() as usize + 1).min(last_row_index);
    let mut y_offset = start_offset as f32 * row_height_plus_spacing;
    for (i, row) in rows.skip(start_offset).take(num_rows_to_render).enumerate() {
      for (mapper, base_layout) in mappers.iter().zip(layout.children()) {
        let element = mapper(&row);
        // HACK: Reconstruct the layout from `base_layout` which has a correct x position, but an incorrect y position
        //       which always points to the first row. This is needed so that we do not have to lay out all the cells of
        //       the table each time the layout changes, because that is slow for larger tables. Instead we now
        //       calculate the absolute layout of cells which are *in view* as part of primitive generation.
        let bounds = base_layout.bounds();
        let mut node = Node::new(Size::new(bounds.width, bounds.height));
        node.move_to(Point::new(bounds.x, absolute_position.y + y_offset));
        let layout = Layout::new(&node);
        let (primitive, new_mouse_cursor) = element.draw(self, defaults, layout, cursor_position, viewport);
        if new_mouse_cursor > mouse_cursor { mouse_cursor = new_mouse_cursor; }
        primitives.push(primitive);
      }
      y_offset += row_height;
      if i < last_row_index { // Don't add spacing after last row.
        y_offset += spacing;
      }
    }
    (Primitive::Group { primitives }, mouse_cursor)
  }
}

impl<'a, T, I, M, R> Into<Element<'a, M, R>> for TableRows<'a, T, I, M, R> where
  T: 'a,
  I: 'a + Iterator<Item=T> + ExactSizeIterator + Clone,
  M: 'a,
  R: 'a + TableRowsRenderer<'a, T, I, M>,
{
  fn into(self) -> Element<'a, M, R> {
    Element::new(self)
  }
}

//
// Column layout calculation
//

fn layout_columns(total_width: f32, row_height: f32, width_fill_portions: &Vec<u32>, spacing: u32) -> Vec<Node> {
  let num_columns = width_fill_portions.len();
  let last_column_index = num_columns.saturating_sub(1);
  let num_spacers = last_column_index;
  let total_spacing = (spacing as usize * num_spacers) as f32;
  let total_space = total_width - total_spacing;
  let total_fill_portion = width_fill_portions.iter().sum::<u32>() as f32;
  let mut layouts = Vec::new();
  let mut x_offset = 0f32;
  for (i, width_fill_portion) in width_fill_portions.iter().enumerate() {
    let width = (*width_fill_portion as f32 / total_fill_portion) * total_space;
    let mut layout = Node::new(Size::new(width, row_height));
    layout.move_to(Point::new(x_offset, 0f32));
    layouts.push(layout);
    x_offset += width;
    if i < last_column_index { // Don't add spacing after last column.
      x_offset += spacing as f32;
    }
  }
  layouts
}
