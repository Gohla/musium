use tui::{
  backend::Backend,
  Frame,
  layout::{Constraint, Layout},
  style::{Color, Style},
  text::{Span, Spans},
  widgets::{Block, Borders, Tabs},
};

use crate::app::App;

pub fn draw<B: Backend>(_f: &mut Frame<B>, _app: &mut App) {

}
