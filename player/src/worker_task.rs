use std::error::Error as StdError;
use std::sync::{Arc, RwLock};

use thiserror::Error;
use tokio::{self, sync::{mpsc, oneshot, watch}, task};
use tracing::{event, instrument, Level};

use musium_core::api::PlaySourceKind;

use crate::{AudioOutputT, ClientT};



enum Request {
  PlayTrack { id: i32, tx: oneshot::Sender<Result<(), PlayError>> },
  TogglePlay { tx: oneshot::Sender<Result<(), PlayError>> },
  Stop { tx: oneshot::Sender<Result<(), PlayError>> },
}

struct WorkerTask {
  client: super::Client,
  audio_output: super::AudioOutput,
  rx: mpsc::Receiver<Request>,
  current_track: Option<CurrentTrack>,
}

struct CurrentTrack {
  id: i32,
  play_source_kind: PlaySourceKind,
}

impl WorkerTask {
  fn new(
    client: super::Client,
    audio_output: super::AudioOutput,
    rx: mpsc::Receiver<Request>,
  ) -> Self {
    Self {
      client,
      audio_output,
      rx,
      current_track: None,
    }
  }

  #[instrument(skip(self))]
  async fn run(mut self) {
    while let Some(request) = self.rx.recv().await { // Loop until all senders disconnect.
      match request {
        Request::PlayTrack { id, tx } => {
          use PlayError::*;
          use musium_core::api::PlaySource::*;
          let (play_source, play_source_kind) = self.get_client().play_track_by_id(id).await.map_err(|e| ClientFail(e))?;
          match play_source {
            Some(AudioData(audio_data)) => self.get_audio_output().set_audio_data(audio_data, true).await.map_err(|e| AudioOutputFail(e))?,
            Some(ExternallyPlayedOnSpotify) => {}
            None => {}
          };
          self.current_track = Some(CurrentTrack { id, play_source_kind });
          Ok(())
        }
      }
    }
  }
}
