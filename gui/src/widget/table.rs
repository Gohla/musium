#![allow(dead_code)]

use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

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

pub struct TableBuilder<'a, T, M, R> where
  T: 'a,
{
  width: Length,
  height: Length,
  max_width: u32,
  max_height: u32,
  spacing: u32,
  header: TableHeader<'a, M, R>,
  rows: TableRows<'a, T, M, R>,
}

impl<'a, T, M, R> TableBuilder<'a, T, M, R> where
  T: 'a,
{
  pub fn new(rows: Rc<RefCell<Vec<T>>>) -> Self {
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


  pub fn push_column<E>(mut self, width_fill_portion: u32, header: E, mapper: Box<dyn 'a + Fn(&mut T) -> Element<'_, M, R>>) -> Self where
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
    R: 'a + TableRenderer + TableRowsRenderer<'a, T, M>
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

struct TableRows<'a, T, M, R> where
  T: 'a,
{
  spacing: u32,
  row_height: u32,
  column_fill_portions: Vec<u32>,
  mappers: Vec<Box<dyn 'a + Fn(&mut T) -> Element<'_, M, R>>>,
  // HACK: store row data as `Rc<RefCell<Vec<T>>>` because I bashed my head in for hours trying to get the lifetimes
  //       and mutability right with a more general type. The `RefCell` is needed because the `mappers` want a `&mut` to
  //       row data, so that they can return mutable state such as button states, but we do not have `&mut self` in the
  //       `draw` method, making it impossible to get an exclusive borrow to `rows`. Therefore, `Rc` is needed to share
  //       the data with the owner of this widget. The `Vec` is needed because that is what the owner of this widget
  //       provides, and I could not figure out how to take a more general type/trait inside `Rc`/`RefCell`.
  //
  //       Ideally, we want to take something like `T: 'a, I: 'a + IntoIterator, I::Item=&'a mut T,
  //       I::IntoIter='a + ExactSizeIterator`.
  rows: Rc<RefCell<Vec<T>>>,
}

impl<'a, T, M, R: TableRowsRenderer<'a, T, M>> Widget<M, R> for TableRows<'a, T, M, R> where
  T: 'a,
{
  fn width(&self) -> Length { Length::Fill }
  fn height(&self) -> Length { Length::Fill }

  fn layout(&self, _renderer: &R, limits: &Limits) -> Node {
    let max = limits.max();
    let total_width = max.width;
    // HACK: only lay out first row, because laying out the entire table becomes slow for larger tables. Reconstruct
    //       the layout of elements on-demand with `reconstruct_layout_node`.
    let layouts = layout_columns(total_width, self.row_height as f32, &self.column_fill_portions, self.spacing);
    let num_rows = self.rows.borrow().len();
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
    renderer.draw_table_rows(defaults, layout, cursor_position, viewport, self.row_height as f32, self.spacing as f32, &self.mappers, &mut self.rows.borrow_mut())
  }

  fn hash_layout(&self, state: &mut Hasher) {
    struct Marker;
    std::any::TypeId::of::<Marker>().hash(state);
    self.spacing.hash(state);
    self.row_height.hash(state);
    for column_fill_portion in &self.column_fill_portions {
      column_fill_portion.hash(state);
    }
    self.rows.borrow().len().hash(state);
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
        if self.propagate_event_to_element_at(&event, mouse_position_relative, layout, cursor_position, messages, renderer, clipboard) == Status::Captured {
          return Status::Captured;
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
        if self.propagate_event_to_element_at(&event, touch_position_relative, layout, cursor_position, messages, renderer, clipboard) == Status::Captured {
          return Status::Captured;
        }
      }
    }
    Status::Ignored
  }
}

impl<'a, T, M, R: TableRowsRenderer<'a, T, M>> TableRows<'a, T, M, R> where
  T: 'a,
{
  fn get_row_index_at(&mut self, y: f32) -> Option<usize> {
    if y < 0f32 { return None; } // Out of bounds
    let spacing = self.spacing as f32;
    let row_height = self.row_height as f32;
    let row_height_plus_spacing = row_height + spacing;
    let row_offset = (y / row_height_plus_spacing).ceil() as usize;
    let row_offset_without_spacing = (row_offset as f32 * row_height_plus_spacing) - spacing;
    if y > row_offset_without_spacing {
      None // On row spacing
    } else {
      Some(row_offset.saturating_sub(1)) // NOTE: + 1 because row_offset is 1-based. Why is this the case?
    }
  }

  fn get_column_index_and_layout_at<'l>(&mut self, x: f32, layout: &Layout<'l>) -> Option<(usize, Layout<'l>)> {
    let spacing = self.spacing as f32;
    let mut offset = 0f32;
    for (column_index, column_layout) in layout.children().enumerate() {
      if x < offset { return None; } // On column spacing or out of bounds
      offset += column_layout.bounds().width;
      if x <= offset { return Some((column_index, column_layout)); }
      offset += spacing;
    }
    None
  }

  fn propagate_event_to_element_at(
    &mut self,
    event: &Event,
    point: Point,
    layout: Layout<'_>,
    cursor_position: Point,
    messages: &mut Vec<M>,
    renderer: &R,
    clipboard: Option<&dyn Clipboard>,
  ) -> Status {
    let absolute_position = layout.position();
    let row_height_plus_spacing = self.row_height as f32 + self.spacing as f32;
    let column_index_and_layout = self.get_column_index_and_layout_at(point.x, &layout);
    let row_index = self.get_row_index_at(point.y);
    if let (Some((column_index, base_layout)), Some(row_index)) = (column_index_and_layout, row_index) {
      let mapper = self.mappers.get(column_index);
      let mut rows_borrow = self.rows.borrow_mut();
      let row = rows_borrow.get_mut(row_index);
      if let (Some(mapper), Some(row)) = (mapper, row) {
        let mut element = mapper(row);
        let y_offset = absolute_position.y + row_index as f32 * row_height_plus_spacing;
        // HACK: reconstruct layout of element to fix its y position based on `y_offset`.
        let node = reconstruct_layout_node(base_layout, y_offset, &element, &renderer);
        let layout = Layout::new(&node);
        element.on_event(event.clone(), layout, cursor_position, messages, renderer, clipboard)
      } else {
        Status::Ignored
      }
    } else {
      Status::Ignored
    }
  }
}

pub trait TableRowsRenderer<'a, T, M>: Renderer where
  T: 'a,
{
  fn draw_table_rows(
    &mut self,
    defaults: &Self::Defaults,
    layout: Layout<'_>,
    cursor_position: Point,
    viewport: &Rectangle<f32>,
    row_height: f32,
    spacing: f32,
    mappers: &[Box<dyn 'a + Fn(&mut T) -> Element<'_, M, Self>>],
    rows: &mut [T],
  ) -> Self::Output;
}

impl<'a, T, M, B> TableRowsRenderer<'a, T, M> for ConcreteRenderer<B> where
  T: 'a,
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
    mappers: &[Box<dyn 'a + Fn(&mut T) -> Element<'_, M, Self>>],
    rows: &mut [T],
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
    // NOTE: + 1 on next line to ensure that last partially visible row is not culled. Why is this needed?
    let num_rows_to_render = ((viewport.height / row_height_plus_spacing).ceil() as usize + 1).min(last_row_index);
    let mut y_offset = absolute_position.y + start_offset as f32 * row_height_plus_spacing;
    for (i, row) in rows.into_iter().skip(start_offset).take(num_rows_to_render).enumerate() {
      for (mapper, base_layout) in mappers.iter().zip(layout.children()) {
        let element: Element<'_, M, Self> = mapper(row);
        // HACK: reconstruct layout of element to fix its y position based on `y_offset`.
        let node = reconstruct_layout_node(base_layout, y_offset, &element, &self);
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

impl<'a, T, M, R> Into<Element<'a, M, R>> for TableRows<'a, T, M, R> where
  T: 'a,
  M: 'a,
  R: 'a + TableRowsRenderer<'a, T, M>,
{
  fn into(self) -> Element<'a, M, R> {
    Element::new(self)
  }
}

//
// Column layout calculation and reconstruction.
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

fn reconstruct_layout_node<M, R: Renderer>(
  base_layout: Layout<'_>,
  y_offset: f32,
  element: &Element<'_, M, R>,
  renderer: &R,
) -> Node {
  // HACK: Reconstruct the layout from `base_layout` which has a correct x position, but an incorrect y position
  //       which always points to the first row. This is needed so that we do not have to lay out all the cells of
  //       the table each time the layout changes, because that is slow for larger tables.
  let bounds = base_layout.bounds();
  let size = Size::new(bounds.width, bounds.height);
  let limits = Limits::new(Size::ZERO, size);
  let mut node = element.layout(renderer, &limits);
  node.move_to(Point::new(bounds.x, y_offset));
  node
}
