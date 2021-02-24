use async_trait::async_trait;

use musium_audio_output::AudioOutput;
use musium_client::Client;
use musium_core::model::{User, UserLogin};

#[async_trait]
pub trait PlayerT {
  type Client: Client;
  type AudioOutput: AudioOutput;

  fn get_client(&self) -> &Self::Client;

  fn get_audio_output(&self) -> &Self::AudioOutput;


  async fn login(&self, user_login: &UserLogin) -> Result<User, <Self::Client as Client>::LoginError> {
    self.get_client().login(user_login).await
  }

  async fn play_track_by_id(&self, id: i32, volume: f32) -> Result<(), <Self::Client as Client>::LoginError> {
    use musium_client::PlaySource::*;
    let play_source = self.get_client().play_track_by_id(id).await?;
    match play_source {
      Some(AudioData(audio_data)) => self.get_audio_output().play(audio_data, volume).await?,
      Some(ExternallyPlayed) => {}
      None => {}
    };
    Ok(())
  }
}
