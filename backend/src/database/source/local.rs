use diesel::prelude::*;

use musium_core::model::{LocalSource, NewLocalSource};
use musium_core::schema;

use crate::database::{DatabaseConnection, DatabaseQueryError};

impl DatabaseConnection {
  pub fn list_local_sources(&self) -> Result<Vec<LocalSource>, DatabaseQueryError> {
    use schema::local_source::dsl::*;
    Ok(time!("list_local_sources.select", local_source.load::<LocalSource>(&self.connection)?))
  }

  pub fn get_local_source_by_id(&self, local_source_id: i32) -> Result<Option<LocalSource>, DatabaseQueryError> {
    let query = {
      use schema::local_source::dsl::*;
      local_source.find(local_source_id)
    };
    Ok(time!("get_local_source_by_id.select", query.first::<LocalSource>(&self.connection).optional()?))
  }

  pub fn create_or_enable_local_source(&self, new_local_source: &NewLocalSource) -> Result<LocalSource, DatabaseQueryError> {
    let select_by_directory_query = {
      use schema::local_source::dsl::*;
      local_source.filter(directory.eq(&new_local_source.directory))
    };
    let db_local_source: Option<LocalSource> = time!("create_or_enable_local_source.select", select_by_directory_query.first::<LocalSource>(&self.connection).optional()?);
    Ok(if let Some(mut db_local_source) = db_local_source {
      if !db_local_source.enabled {
        db_local_source.enabled = true;
        time!("create_or_enable_local_source.update", db_local_source.save_changes::<LocalSource>(&*self.connection)?);
      }
      db_local_source
    } else {
      // TODO: must be done in transaction for consistency.
      let insert_query = {
        use schema::local_source::dsl::*;
        diesel::insert_into(local_source).values(new_local_source)
      };
      time!("create_or_enable_local_source.insert", insert_query.execute(&self.connection)?);
      let select_query = {
        use schema::local_source::dsl::*;
        local_source.order(id.desc()).limit(1)
      };
      time!("create_or_enable_local_source.select_inserted", select_query.first::<LocalSource>(&self.connection)?)
    })
  }

  pub fn set_local_source_enabled_by_id(&self, local_source_id: i32, enabled: bool) -> Result<Option<LocalSource>, DatabaseQueryError> {
    let local_source = {
      use schema::local_source::dsl::*;
      time!("set_local_source_enabled_by_id.select", local_source.find(local_source_id).first::<LocalSource>(&self.connection).optional()?)
    };
    if let Some(mut local_source) = local_source {
      local_source.enabled = enabled;
      time!("set_local_source_enabled_by_id.update", local_source.save_changes::<LocalSource>(&*self.connection)?);
      Ok(Some(local_source))
    } else {
      Ok(None)
    }
  }

  pub fn enable_local_source_by_id(&self, local_source_id: i32) -> Result<Option<LocalSource>, DatabaseQueryError> {
    self.set_local_source_enabled_by_id(local_source_id, true)
  }

  pub fn disable_local_source_by_id(&self, local_source_id: i32) -> Result<Option<LocalSource>, DatabaseQueryError> {
    self.set_local_source_enabled_by_id(local_source_id, false)
  }
}
