use musium_client::Client;
use musium_audio_output::AudioOutput;
use musium_core::model::{UserLogin, User};

use async_trait::async_trait;

#[async_trait]
pub trait Player {
  type Client: Client;
  type AudioOutput: AudioOutput;

  fn get_client(&self) -> &Self::Client;

  fn get_audio_output(&self) -> &Self::AudioOutput;

  async fn login(&self, user_login: &UserLogin) -> Result<User, <Self::Client as Client>::LoginError> {
    self.get_client().login(user_login).await
  }
}
