use thiserror::Error;

pub use musium_audio_output::AudioOutput as AudioOutputT;
#[cfg(feature = "musium_audio_output_rodio")]
pub use musium_audio_output_rodio::{RodioAudioOutput, RodioSetAudioDataError};
pub use musium_client::Client as ClientT;
#[cfg(feature = "musium_client_http")]
pub use musium_client_http::{HttpClient, HttpRequestError, Url};
use musium_core::model::{User, UserLogin};

//mod worker_task;

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
  pub fn new(client: Client, audio_output: AudioOutput) -> Self {
    Self {
      client,
      audio_output,
    }
  }
}

// Destruction

impl Player {
  /// Converts this player into its client and audio output, effectively destroying the player. The client and audio
  /// output can then be manually destroyed.
  ///
  /// Dropping this player and all its clones will also properly destroy it, but ignores the panics produced by the
  /// client and audio output (if any), and also does not wait for them to complete first (if they have tasks or threads
  /// that must be completed).
  pub fn into_client_and_audio_output(self) -> (Client, AudioOutput) {
    (self.client, self.audio_output)
  }
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
  AudioOutputFail(#[source] <AudioOutput as AudioOutputT>::SetAudioDataError),
}

impl Player {
  pub async fn play_track_by_id(&self, id: i32) -> Result<(), PlayError> {
    use PlayError::*;
    use musium_core::api::PlaySource::*;
    let play_source = self.get_client().play_track_by_id(id).await.map_err(|e| ClientFail(e))?;
    match play_source {
      Some(AudioData { data: audio_data }) => self.get_audio_output().set_audio_data(audio_data, true).await.map_err(|e| AudioOutputFail(e))?,
      Some(ExternallyPlayedOnSpotify) => {}
      None => {}
    };
    Ok(())
  }

  //pub async fn toggle_play(&self) -> Result<(), ()> {}
}
