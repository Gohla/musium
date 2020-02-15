#[derive(Clone, PartialOrd, PartialEq, Debug, Queryable)]
pub struct Track {
  pub id: i32,
  pub disc_number: Option<i32>,
  pub disc_total: Option<i32>,
  pub track_number: Option<i32>,
  pub track_total: Option<i32>,
  pub title: String,
  pub file: String,
}
