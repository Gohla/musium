use musium_audio_output::AudioOutput;
use musium_client::Client;
use musium_core::model::{User, UserLogin};

use async_trait::async_trait;
use thiserror::Error;

use std::error::Error as StdError;

#[async_trait]
pub trait PlayerT {
  type Client: Client;
  type AudioOutput: AudioOutput;

  fn get_client(&self) -> &Self::Client;

  fn get_client_mut(&mut self) -> &mut Self::Client;

  fn get_audio_output(&self) -> &Self::AudioOutput;

  fn get_audio_output_mut(&mut self) -> &mut Self::AudioOutput;


  async fn login(&mut self, user_login: &UserLogin) -> Result<User, <Self::Client as Client>::LoginError> {
    self.get_client_mut().login(user_login).await
  }

  async fn play_track_by_id(&mut self, id: i32, volume: f32) -> Result<(), PlayError<<Self::Client as Client>::TrackError, <Self::AudioOutput as AudioOutput>::PlayError>> {
    use musium_client::PlaySource::*;
    let play_source = self.get_client_mut().play_track_by_id(id).await.map_err(|e| PlayError::ClientFail(e))?;
    match play_source {
      Some(AudioData(audio_data)) => self.get_audio_output_mut().play(audio_data, volume).await.map_err(|e| PlayError::AudioOutputFail(e))?,
      Some(ExternallyPlayed) => {}
      None => {}
    };
    Ok(())
  }
}

#[derive(Debug, Error)]
pub enum PlayError<C: 'static + StdError + Send + Sync, A: 'static + StdError + Send + Sync> {
  #[error(transparent)]
  ClientFail(C),
  #[error(transparent)]
  AudioOutputFail(A),
}
