use std::sync::{Arc, RwLock, RwLockReadGuard};

use thiserror::Error;

pub use musium_audio_output::AudioOutput as AudioOutputT;
#[cfg(feature = "musium_audio_output_rodio")]
pub use musium_audio_output_rodio::{RodioAudioOutput, RodioPlayError};
pub use musium_client::Client as ClientT;
#[cfg(feature = "musium_client_http")]
pub use musium_client_http::{HttpClient, HttpRequestError, Url};
use musium_core::model::{User, UserLogin};
use musium_core::model::collection::Tracks;

#[cfg(feature = "musium_client_http")]
pub type Client = HttpClient;
#[cfg(feature = "musium_audio_output_rodio")]
pub type AudioOutput = RodioAudioOutput;

#[derive(Clone)]
pub struct Player {
  client: Client,
  audio_output: AudioOutput,
  library: Arc<RwLock<Tracks>>,
}

// Creation

impl Player {
  pub fn new(client: Client, audio_output: AudioOutput) -> Self {
    Self {
      client,
      audio_output,
      library: Arc::new(RwLock::new(Tracks::default())),
    }
  }
}

// Destruction

impl Player {
  /// Converts this player into its client and audio output, effectively destroying the player. The client and audio
  /// output can then be manually destroyed.
  ///
  /// Dropping this player and all its clones will also properly destroy this player, but may ignore panics in the
  /// worker threads of the client and audio output (if any).
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

// Library

#[derive(Debug, Error)]
pub enum RefreshLibraryFail {
  #[error("Failed to get library from the client")]
  ClientFail(#[source] <Client as ClientT>::TrackError),
}

impl<'a> Player {
  pub async fn refresh_library(&'a self) -> Result<RwLockReadGuard<'a, Tracks>, RefreshLibraryFail> {
    use RefreshLibraryFail::*;
    let library_raw = self.get_client().list_tracks().await.map_err(|e| ClientFail(e))?;
    // UNWRAP: returns error when thread panicked. In that case we also panic.
    let library: Tracks = tokio::task::spawn_blocking(|| { library_raw.into() }).await.unwrap();
    {
      // UNWRAP: returns error when lock is poisoned, which is caused by a panic. In that case we also panic.
      let mut library_write_locked = self.library.write().unwrap();
      *library_write_locked = library;
    }
    // UNWRAP: returns error when lock is poisoned, which is caused by a panic. In that case we also panic.
    let library_read_locked = self.library.read().unwrap();
    Ok(library_read_locked)
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
