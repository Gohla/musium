use thiserror::Error;

pub use musium_audio_output::AudioOutput as AudioOutputT;
#[cfg(feature = "musium_audio_output_rodio")]
pub use musium_audio_output_rodio::{RodioAudioOutput, RodioPlayError};
pub use musium_client::Client as ClientT;
#[cfg(feature = "musium_client_http")]
pub use musium_client_http::{HttpClient, HttpRequestError, Url};
use musium_core::model::{User, UserLogin};

#[cfg(feature = "musium_client_http")]
pub type Client = HttpClient;
#[cfg(feature = "musium_audio_output_rodio")]
pub type AudioOutput = RodioAudioOutput;

#[derive(Clone)]
pub struct Player {
  client: Client,
  audio_output: AudioOutput,
}

// Creation

impl Player {
  pub fn new(client: Client, audio_output: AudioOutput) -> Self { Self { client, audio_output } }
}

// Getters

impl Player {
  #[inline]
  pub fn get_client(&self) -> &Client { &self.client }

  #[inline]
  pub fn get_audio_output(&self) -> &AudioOutput { &self.audio_output }
}

// Login

impl Player {
  pub async fn login(&self, user_login: &UserLogin) -> Result<User, <Client as ClientT>::LoginError> {
    self.get_client().login(user_login).await
  }
}

// Playback

#[derive(Debug, Error)]
pub enum PlayError {
  #[error("Failed to get playback data from the client")]
  ClientFail(#[source] <Client as ClientT>::TrackError),
  #[error("Failed to play audio data with the audio output")]
  AudioOutputFail(#[source] <AudioOutput as AudioOutputT>::PlayError),
}

impl Player {
  pub async fn play_track_by_id(&self, id: i32, volume: f32) -> Result<(), PlayError> {
    use PlayError::*;
    use musium_client::PlaySource::*;
    let play_source = self.get_client().play_track_by_id(id).await.map_err(|e| ClientFail(e))?;
    match play_source {
      Some(AudioData(audio_data)) => self.get_audio_output().play(audio_data, volume).await.map_err(|e| AudioOutputFail(e))?,
      Some(ExternallyPlayed) => {}
      None => {}
    };
    Ok(())
  }
}
