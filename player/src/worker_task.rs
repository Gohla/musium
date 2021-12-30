use std::time::Duration;

use tokio::select;
use tokio::sync::{mpsc, oneshot};
use tokio::time;
use tokio::time::{Instant, MissedTickBehavior};

use musium_audio_output::AudioOutput;
use musium_client::Client;
use musium_core::model::{User, UserLogin};

#[derive(Debug, Clone)]
struct WorkerTask<C: Client, AO: AudioOutput> {
  client: C,
  audio_output: AO,
  request_rx: mpsc::UnboundedReceiver<Request<C, AO>>,
  event_tx: mpsc::UnboundedSender<Event<C, AO>>,
  queue: Vec<i32>,
  queue_index: Option<usize>,
}

enum Request<C: Client, AO: AudioOutput> {
  Login { user_login: UserLogin, tx: oneshot::Sender<Result<User, C::LoginError>> },
  SetQueue { queue: Vec<i32>, tx: oneshot::Sender<()> },
  Play { tx: oneshot::Sender<Result<(), AO::PlayError>> },
  TogglePlay { tx: oneshot::Sender<Result<bool, AO::TogglePlayError>> },
  Stop { tx: oneshot::Sender<Result<(), AO::StopError>> },
}

pub enum Event<C: Client, AO: AudioOutput> {}

impl<C: Client, AO: AudioOutput> WorkerTask<C, AO> {
  fn new(
    client: C,
    audio_output: AO,
  ) -> (Self, mpsc::UnboundedSender<Request<C, AO>>, mpsc::UnboundedReceiver<Event<C, AO>>) {
    let (request_tx, request_rx) = mpsc::unbounded_channel();
    let (event_tx, event_rx) = mpsc::unbounded_channel();
    let worker_task = Self {
      client,
      audio_output,
      request_rx,
      event_tx,
      queue: Vec::new(),
      queue_index: None,
    };
    (worker_task, request_tx, event_rx)
  }

  async fn run(mut self) {
    let mut interval = time::interval(Duration::from_millis(10));
    interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
      select! {
        maybe_request = self.request_rx.recv() => {
          if let Some(request) = maybe_request {
            self.handle_request(request).await;
          } else {
            break; // Sender hung up; break out of loop to end the task.
          }
        }
        instant = interval.tick(), if self.queue_index.is_some() => {
          self.handle_interval(instant).await;
        }
      }
    }
  }

  async fn handle_request(&mut self, request: Request<C, AO>) {
    match request { // `ok` in `tx.send`: receiver hung up -> we don't care.
      Request::Login { user_login, tx } => {
        tx.send(self.client.login(&user_login).await).ok();
      }
      Request::SetQueue { queue: playlist, tx } => {
        self.queue = playlist;
        tx.send(()).ok();
      }
      Request::Play { tx } => {
        tx.send(self.audio_output.play().await).ok();
      }
      Request::TogglePlay { tx } => {
        tx.send(self.audio_output.toggle_play().await).ok();
      }
      Request::Stop { tx } => {
        tx.send(self.audio_output.stop().await).ok();
      }
    }
  }

  async fn handle_interval(&mut self, _instant: Instant) {
    if let Some(queue_index) = self.queue_index {
      if queue_index == self.queue.len() - 1 {
        return;
      }
      match self.audio_output.is_stopped() {
        Ok(true) => {
          self.queue_index += 1;
          self.play_track_by_id(self.queue[self.queue_index]); // TODO: send event
        }
      }
    }
  }


  async fn play_track_by_id(&self, id: i32) -> Result<(), Self::PlayError> {
    use PlayError::*;
    use musium_core::api::PlaySource::*;
    let play_source = self.client.play_track_by_id(id).await.map_err(|e| ClientPlayTrackFail(e))?;
    match play_source {
      Some(AudioData { codec, data }) => self.audio_output.set_audio_data(codec, data).await.map_err(|e| SetAudioDataFail(e))?,
      Some(ExternallyPlayedOnSpotify) => {}
      None => {}
    };
    self.audio_output.play().await.map_err(|e| AudioOutputPlayFail(e))?;
    Ok(())
  }
}


// use std::error::Error as StdError;
// use std::sync::{Arc, RwLock};
//
// use thiserror::Error;
// use tokio::{self, sync::{mpsc, oneshot, watch}, task};
// use tracing::{event, instrument, Level};
//
// use musium_core::api::PlaySourceKind;
//
// use crate::{AudioOutputT, ClientT};
//
//
//
// enum Request {
//   PlayTrack { id: i32, tx: oneshot::Sender<Result<(), PlayError>> },
//   TogglePlay { tx: oneshot::Sender<Result<(), PlayError>> },
//   Stop { tx: oneshot::Sender<Result<(), PlayError>> },
// }
//
// struct WorkerTask {
//   client: super::Client,
//   audio_output: super::AudioOutput,
//   rx: mpsc::Receiver<Request>,
//   current_track: Option<CurrentTrack>,
// }
//
// struct CurrentTrack {
//   id: i32,
//   play_source_kind: PlaySourceKind,
// }
//
// impl WorkerTask {
//   fn new(
//     client: super::Client,
//     audio_output: super::AudioOutput,
//     rx: mpsc::Receiver<Request>,
//   ) -> Self {
//     Self {
//       client,
//       audio_output,
//       rx,
//       current_track: None,
//     }
//   }
//
//   #[instrument(skip(self))]
//   async fn run(mut self) {
//     while let Some(request) = self.rx.recv().await { // Loop until all senders disconnect.
//       match request {
//         Request::PlayTrack { id, tx } => {
//           use PlayError::*;
//           use musium_core::api::PlaySource::*;
//           let (play_source, play_source_kind) = self.get_client().play_track_by_id(id).await.map_err(|e| ClientFail(e))?;
//           match play_source {
//             Some(AudioData(audio_data)) => self.get_audio_output().set_audio_data(audio_data, true).await.map_err(|e| AudioOutputFail(e))?,
//             Some(ExternallyPlayedOnSpotify) => {}
//             None => {}
//           };
//           self.current_track = Some(CurrentTrack { id, play_source_kind });
//           Ok(())
//         }
//       }
//     }
//   }
// }
