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
    let insert_query = {
      use schema::source::dsl::*;
      diesel::insert_into(source).values(&new_source)
    };
    time!("add_source.insert", insert_query.execute(&self.connection)?);
    let select_query = {
      use schema::source::dsl::*;
      source
        .order(id.desc())
        .limit(1)
    };
    Ok(time!("add_source.select_inserted", select_query.first::<Source>(&self.connection)?))
  }

  pub fn set_source_enabled_by_id(&self, source_id: i32, enabled: bool) -> Result<Option<Source>, DatabaseQueryError> {
    let source = {
      use schema::source::dsl::*;
      time!("add_source.select", source.find(source_id).first::<Source>(&self.connection).optional()?)
    };
    if let Some(mut source) = source {
      source.enabled = enabled;
      time!("add_source.update", source.save_changes::<Source>(&*self.connection)?);
      Ok(Some(source))
    } else {
      Ok(None)
    }
  }

  pub fn enable_source_by_id(&self, source_id: i32) -> Result<Option<Source>, DatabaseQueryError> {
    self.set_source_enabled_by_id(source_id, true)
  }

  pub fn disable_source_by_id(&self, source_id: i32) -> Result<Option<Source>, DatabaseQueryError> {
    self.set_source_enabled_by_id(source_id, false)
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
