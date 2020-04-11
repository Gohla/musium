use diesel::prelude::*;

use musium_core::model::{NewSource, Source};
use musium_core::schema;

use super::{DatabaseConnection, DatabaseQueryError};

impl DatabaseConnection<'_> {
  pub fn list_sources(&self) -> Result<Vec<Source>, DatabaseQueryError> {
    use schema::source::dsl::*;
    Ok(source.load::<Source>(&self.connection)?)
  }

  pub fn get_source_by_id(&self, source_id: i32) -> Result<Option<Source>, DatabaseQueryError> {
    use schema::source::dsl::*;
    Ok(source.find(source_id).first::<Source>(&self.connection).optional()?)
  }

  pub fn create_source(&self, new_source: NewSource) -> Result<Source, DatabaseQueryError> {
    let select_query = {
      use schema::source::dsl::*;
      source
        .filter(directory.eq(&new_source.directory))
    };
    let source = time!("add_source.select", select_query.first::<Source>(&self.connection).optional()?);
    Ok(if let Some(mut source) = source {
      // Enable existing source.
      source.enabled = new_source.enabled;
      time!("add_source.update", source.save_changes::<Source>(&*self.connection)?);
      source
    } else {
      // Insert new scan directory.
      let insert_query = {
        use schema::source::dsl::*;
        diesel::insert_into(source)
          .values(&new_source)
      };
      time!("add_source.insert", insert_query.execute(&self.connection)?);
      time!("add_source.select_inserted", select_query.first::<Source>(&self.connection)?)
    })
  }

  pub fn delete_source_by_id(&self, source_id: i32) -> Result<bool, DatabaseQueryError> {
    let select_query = {
      use schema::source::dsl::*;
      source.find(source_id)
    };
    let source = time!("remove_source.select", select_query.first::<Source>(&self.connection).optional()?);
    if let Some(mut source) = source {
      source.enabled = false;
      time!("remove_source.update", source.save_changes::<Source>(&*self.connection)?);
      Ok(true)
    } else {
      Ok(false)
    }
  }
}
