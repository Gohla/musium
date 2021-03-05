use std::backtrace::Backtrace;

use diesel::prelude::*;
use thiserror::Error;

use musium_core::model::{NewUser, NewUserAlbumRating, NewUserArtistRating, NewUserTrackRating, User, UserAlbumRating, UserArtistRating, UserLogin, UserTrackRating};
use musium_core::schema;

use crate::model::{InternalNewUser, InternalUser};

use super::{DatabaseConnection, DatabaseQueryError};

// User database queries

#[derive(Debug, Error)]
pub enum UserAddVerifyError {
  #[error("Failed to execute a database query")]
  DatabaseQueryFail(#[from] diesel::result::Error, Backtrace),
  #[error("Failed to hash password")]
  PasswordHashFail(#[from] crate::password::HashError, Backtrace),
}

impl DatabaseConnection {
  pub fn list_users(&self) -> Result<Vec<User>, DatabaseQueryError> {
    use schema::user::dsl::*;
    Ok(user.select((id, name)).load::<User>(&self.connection)?)
  }

  pub fn get_user_by_id(&self, input_id: i32) -> Result<Option<User>, DatabaseQueryError> {
    use schema::user::dsl::*;
    Ok(user.select((id, name)).find(input_id).first::<User>(&self.connection).optional()?)
  }

  pub fn verify_user(&self, user_login: &UserLogin) -> Result<Option<User>, UserAddVerifyError> {
    let user: Option<InternalUser> = {
      use schema::user::dsl::*;
      user
        .filter(name.eq(&user_login.name))
        .first::<InternalUser>(&self.connection)
        .optional()?
    };
    if let Some(user) = user {
      if self.inner.password_hasher.verify(&user_login.password, &user.salt, &user.hash)? {
        Ok(Some(user.into()))
      } else {
        Ok(None)
      }
    } else {
      Ok(None)
    }
  }

  pub fn create_user(&self, new_user: NewUser) -> Result<User, UserAddVerifyError> {
    use schema::user;
    let salt = self.inner.password_hasher.generate_salt();
    let hash = self.inner.password_hasher.hash(new_user.password, &salt)?;
    let internal_new_user = InternalNewUser {
      name: new_user.name.clone(),
      hash,
      salt,
    };
    time!("create_user.insert", diesel::insert_into(user::table)
      .values(internal_new_user)
      .execute(&self.connection)?);
    let select_query = user::table
      .select((user::id, user::name))
      .filter(user::name.eq(&new_user.name));
    Ok(time!("create_user.select", select_query.first::<User>(&self.connection)?))
  }

  pub fn delete_user_by_name<S: AsRef<str>>(&self, name: S) -> Result<bool, DatabaseQueryError> {
    use schema::user;
    let name = name.as_ref();
    let result = time!("delete_user_by_name.delete", diesel::delete(user::table.filter(user::name.eq(name)))
      .execute(&self.connection)?);
    Ok(result == 1)
  }

  pub fn delete_user_by_id(&self, input_id: i32) -> Result<bool, DatabaseQueryError> {
    use schema::user;
    let result = time!("delete_user_by_id.delete", diesel::delete(user::table.filter(user::id.eq(input_id)))
      .execute(&self.connection)?);
    Ok(result == 1)
  }
}


// User data database queries

impl DatabaseConnection {
  pub fn set_user_album_rating(&self, user_id: i32, album_id: i32, rating: i32) -> Result<UserAlbumRating, DatabaseQueryError> {
    use schema::user_album_rating;
    let select_query = user_album_rating::table
      .filter(user_album_rating::user_id.eq(user_id))
      .filter(user_album_rating::album_id.eq(album_id));
    let db_user_album_rating = time!("set_user_album_rating.select", select_query.first::<UserAlbumRating>(&self.connection).optional()?);
    if let Some(db_user_album_rating) = db_user_album_rating {
      let mut db_user_album_rating: UserAlbumRating = db_user_album_rating;
      db_user_album_rating.rating = rating;
      Ok(time!("set_user_album_rating.update", db_user_album_rating.save_changes(&*self.connection)?))
    } else {
      time!("set_user_album_rating.insert", diesel::insert_into(user_album_rating::table)
        .values(NewUserAlbumRating { user_id, album_id, rating })
        .execute(&self.connection)?);
      Ok(time!("set_user_album_rating.select_inserted", select_query.first::<UserAlbumRating>(&self.connection)?))
    }
  }

  pub fn set_user_track_rating(&self, user_id: i32, track_id: i32, rating: i32) -> Result<UserTrackRating, DatabaseQueryError> {
    use schema::user_track_rating;
    let select_query = user_track_rating::table
      .filter(user_track_rating::user_id.eq(user_id))
      .filter(user_track_rating::track_id.eq(track_id));
    let db_user_track_rating = time!("set_user_track_rating.select", select_query.first::<UserTrackRating>(&self.connection).optional()?);
    if let Some(db_user_track_rating) = db_user_track_rating {
      let mut db_user_track_rating: UserTrackRating = db_user_track_rating;
      db_user_track_rating.rating = rating;
      Ok(time!("set_user_track_rating.update", db_user_track_rating.save_changes(&*self.connection)?))
    } else {
      time!("set_user_track_rating.insert", diesel::insert_into(user_track_rating::table)
        .values(NewUserTrackRating { user_id, track_id, rating })
        .execute(&self.connection)?);
      Ok(time!("set_user_track_rating.select_inserted", select_query.first::<UserTrackRating>(&self.connection)?))
    }
  }

  pub fn set_user_artist_rating(&self, user_id: i32, artist_id: i32, rating: i32) -> Result<UserArtistRating, DatabaseQueryError> {
    use schema::user_artist_rating;
    let select_query = user_artist_rating::table
      .filter(user_artist_rating::user_id.eq(user_id))
      .filter(user_artist_rating::artist_id.eq(artist_id));
    let db_user_artist_rating = time!("set_user_artist_rating.select", select_query.first::<UserArtistRating>(&self.connection).optional()?);
    if let Some(db_user_artist_rating) = db_user_artist_rating {
      let mut db_user_artist_rating: UserArtistRating = db_user_artist_rating;
      db_user_artist_rating.rating = rating;
      Ok(time!("set_user_artist_rating.update", db_user_artist_rating.save_changes(&*self.connection)?))
    } else {
      time!("set_user_artist_rating.insert", diesel::insert_into(user_artist_rating::table)
        .values(NewUserArtistRating { user_id, artist_id, rating })
        .execute(&self.connection)?);
      Ok(time!("set_user_artist_rating.select_inserted", select_query.first::<UserArtistRating>(&self.connection)?))
    }
  }
}
