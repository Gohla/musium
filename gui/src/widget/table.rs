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
  header_row_align: Align,
  row_height: u32,
  rows: Vec<Vec<Element<'a, M, R>>>,
}

pub struct TableColumn<'a, M, R> {
  width: Length,
  header: Element<'a, M, R>,
}

impl<'a, M, R> Table<'a, M, R> {
  pub fn new() -> Self {
    Self {
      width: Length::Shrink,
      height: Length::Shrink,
      max_width: u32::MAX,
      max_height: u32::MAX,
      padding: 0,

      spacing: 0,
      columns: Vec::new(),
      header_row_align: Align::Start,
      row_height: 16,
      rows: Vec::new(),
    }
  }


  pub fn push_column<E>(mut self, width: Length, header: E) -> Self
    where E: Into<Element<'a, M, R>>
  {
    let header = header.into();
    self.columns.push(TableColumn { width, header });
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
    // Adjust limits of the entire table container.
    let limits = limits
      .max_width(self.max_width)
      .max_height(self.max_height)
      .width(self.width)
      .height(self.height)
      .pad(self.padding as f32)
      ;

    // TODO: not use row/container here, just layout ourselves.
    // Layout table header row.
    let header_row_layout = {
      let mut row: Row<'_, M, Renderer<B>> = Row::new()
        .width(Length::Fill)
        .spacing(self.spacing)
        .align_items(self.header_row_align)
        ;
      for column in &self.columns {
        let container: Container<'_, M, Renderer<B>> = Container::new(column.header)
          .width(column.width)
          .height(Length::Units(self.row_height as u16))
          ;
        row = row.push(container);
      }
      row.layout(renderer, &limits)
    };

    // let limits_per_column = {
    //   let mut limits = Vec::new();
    //   for node in header_row_layout.children() {
    //     let size = node.size();
    //     limits.push(Limits::new(Size::ZERO, size));
    //   }
    //   limits
    // };

    // Layout table rows. Use layout from `header_row_layout` to sync layout of header row columns with table columns.
    // TODO: for each cell, setup limits based on `self.row_height`, `column_layout.width`, and spacing. Then lay
    //  out the cell with those limits. Then move the cell to the correct spot. Then move the correct spot further along.
    let rows_layout = {
      let mut x = 0f32;
      let mut y = self.row_height as f32 + self.spacing as f32;
      let mut row_nodes = Vec::new();
      for row in self.rows.iter() {
        for (cell, column_layout) in row.iter().zip(header_row_layout.children().iter()) {
          let column_size = column_layout.size();
          let limits = Limits::new(column_size, column_size);
          let mut node = cell.layout(renderer, &limits);
          node.move_to(Point::new(x, y));
          row_nodes.push(node);
          x += column_size.width + self.spacing as f32; // TODO: this puts spacing on the last element
          y += column_size.height + self.spacing as f32; // TODO: this puts spacing on the last element
        }
      }
      Node::with_children(Size::new(x, y), row_nodes)
    };

    let size = Size::new(header_row_layout.size().width + rows_layout.size().width, header_row_layout.size().height + rows_layout.size().height);
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
      info!("Header cell primitive {:?}", primitive);
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
      info!("Cell primitive {:?}", primitive);
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
    self.header_row_align.hash(state);

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
