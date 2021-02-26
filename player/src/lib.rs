pub use musium_audio_output::AudioOutput;
#[cfg(feature = "musium_audio_output_rodio")]
pub use musium_audio_output_rodio::{RodioAudioOutput, RodioPlayError};
pub use musium_client::Client;
#[cfg(feature = "musium_client_http")]
pub use musium_client_http::{HttpClient, HttpRequestError, Url};

pub use crate::player::PlayerT;

pub mod player;

#[cfg(feature = "musium_client_http")]
pub type ConcreteClient = HttpClient;

#[cfg(feature = "musium_audio_output_rodio")]
pub type ConcreteAudioOutput = RodioAudioOutput;

pub struct Player {
  client: ConcreteClient,
  audio_output: ConcreteAudioOutput,
}

impl Player {
  pub fn new(client: ConcreteClient, audio_output: ConcreteAudioOutput) -> Self { Self { client, audio_output } }
}

impl PlayerT for Player {
  type Client = ConcreteClient;
  type AudioOutput = ConcreteAudioOutput;
  #[inline]
  fn get_client(&self) -> &Self::Client { &self.client }
  #[inline]
  fn get_client_mut(&mut self) -> &mut Self::Client { &mut self.client }
  #[inline]
  fn get_audio_output(&self) -> &Self::AudioOutput { &self.audio_output }
  #[inline]
  fn get_audio_output_mut(&mut self) -> &mut Self::AudioOutput { &mut self.audio_output }
}
