use std::hash::Hash;

use iced_graphics::{Backend, Defaults, Primitive, Renderer};
use iced_native::{Align, Background, Color, Container, Element, Hasher, layout, Layout, layout::flex, Length, mouse, Point, Rectangle, Row, Size, Widget};
use iced_native::layout::{Limits, Node};
use tracing::info;

pub struct Table<'a, M, R> {
  // Properties for the entire table.
  width: Length,
  height: Length,
  max_width: u32,
  max_height: u32,
  padding: u16,
  // Properties for elements inside the table.
  spacing: u16,
  columns: Vec<TableColumn<'a, M, R>>,
  row_height: u32,
  rows: Vec<Vec<Element<'a, M, R>>>,
}

pub struct TableColumn<'a, M, R> {
  width_fill_portion: u16,
  header: Element<'a, M, R>,
}

impl<'a, M, R> Table<'a, M, R> {
  pub fn new() -> Self {
    Self {
      width: Length::Fill,
      height: Length::Fill,
      max_width: u32::MAX,
      max_height: u32::MAX,
      padding: 0,

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

  pub fn padding(mut self, padding: u16) -> Self {
    self.padding = padding;
    self
  }


  pub fn spacing(mut self, spacing: u16) -> Self {
    self.spacing = spacing;
    self
  }

  pub fn push_column<E>(mut self, width_fill_portion: u16, header: E) -> Self
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
      .height(self.height)
      .pad(self.padding as f32)
      ;

    let max_size = limits.max();
    let total_width = max_size.width;

    struct ColumnLimitAndOffset {
      limits: Limits,
      x_offset: f32,
    }

    let (column_limits_and_offsets, header_row_layout) = {
      let num_columns = self.columns.len();
      let num_spacers = num_columns.saturating_sub(1);
      let total_width_spacing = (self.spacing as usize * num_spacers) as f32;
      let total_width_space = total_width - total_width_spacing;
      let total_height = max_size.height.min(self.row_height as f32);
      let total_fill_portion = self.columns.iter().map(|c| c.width_fill_portion).sum::<u16>() as f32;
      let mut limits_and_offsets = Vec::new();
      let mut layouts = Vec::new();
      let mut x_offset = 0f32;
      for (i, column) in self.columns.iter().enumerate() {
        let width = (column.width_fill_portion as f32 / total_fill_portion) * total_width_space;
        let size = Size::new(width, total_height);
        let limits = Limits::new(size, size);
        let mut layout = column.header.layout(renderer, &limits);
        layout.move_to(Point::new(x_offset, 0f32));
        layouts.push(layout);
        limits_and_offsets.push(ColumnLimitAndOffset { limits, x_offset });
        x_offset += width;
        if i < num_columns - 1 {
          x_offset += self.spacing as f32;
        }
      }
      let layout = Node::with_children(Size::new(total_width, total_height), layouts);
      (limits_and_offsets, layout)
    };

    let rows_layout = {
      if self.rows.is_empty() {
        Node::default()
      } else {
        let num_rows = self.rows.len();
        let mut y_offset = self.row_height as f32 + self.spacing as f32;
        let mut row_nodes = Vec::new();
        for (i, row) in self.rows.iter().enumerate() {
          for (cell, ColumnLimitAndOffset { limits, x_offset }) in row.iter().zip(column_limits_and_offsets.iter()) {
            let mut layout = cell.layout(renderer, &limits);
            layout.move_to(Point::new(*x_offset, y_offset));
            row_nodes.push(layout);
          }
          y_offset += self.row_height as f32;
          if i < num_rows - 1 {
            y_offset += self.spacing as f32;
          }
        }
        Node::with_children(Size::new(total_width, y_offset), row_nodes)
      }
    };

    let size = Size::new(total_width, header_row_layout.size().height + rows_layout.size().height);
    Node::with_children(size, vec![header_row_layout, rows_layout])
  }

  fn draw(
    &self,
    renderer: &mut Renderer<B>,
    defaults: &Defaults,
    layout: Layout<'_>,
    cursor_position: Point,
    viewport: &Rectangle<f32>,
  ) -> (Primitive, mouse::Interaction) {
    let mut mouse_cursor = mouse::Interaction::default();
    let mut layout_children = layout.children();

    let mut primitives = Vec::new();

    let header_row_layout = layout_children.next().unwrap();
    for (column, layout) in self.columns.iter().zip(header_row_layout.children()) {
      let (primitive, new_mouse_cursor) = column.header.draw(
        renderer,
        defaults,
        layout,
        cursor_position,
        viewport,
      );
      if new_mouse_cursor > mouse_cursor {
        mouse_cursor = new_mouse_cursor;
      }
      primitives.push(primitive);
    }

    let rows_layout = layout_children.next().unwrap();
    for (element, layout) in self.rows.iter().flat_map(|r| r).zip(rows_layout.children()) {
      let (primitive, new_mouse_cursor) = element.draw(
        renderer,
        defaults,
        layout,
        cursor_position,
        viewport,
      );
      if new_mouse_cursor > mouse_cursor {
        mouse_cursor = new_mouse_cursor;
      }
      primitives.push(primitive);
    }

    (Primitive::Group { primitives }, mouse_cursor)
  }

  fn hash_layout(&self, state: &mut Hasher) {
    struct Marker;
    std::any::TypeId::of::<Marker>().hash(state);

    self.spacing.hash(state);
    self.padding.hash(state);
    self.width.hash(state);
    self.height.hash(state);
    self.max_width.hash(state);
    self.max_height.hash(state);
    // self.header_row_align.hash(state);

    for column in &self.columns {
      column.header.hash_layout(state);
    }
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
