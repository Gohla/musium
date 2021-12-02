use std::fmt::{Debug, Formatter};
use std::io::Cursor;
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;

use async_trait::async_trait;
use rodio::{OutputStream, OutputStreamHandle, Sink};
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};
use tracing::instrument;

pub use musium_audio_output::AudioOutput;
use musium_core::api::AudioCodec;
use musium_core::panic::try_panic_into_string;

#[derive(Clone)]
pub struct RodioAudioOutput {
  tx: mpsc::UnboundedSender<Request>,
  worker_thread: Arc<thread::JoinHandle<()>>,
}

// Creation

#[derive(Debug, Error)]
pub enum RodioCreateError {
  #[error("Failed to create Rodio stream")]
  StreamCreateFail(#[from] rodio::StreamError),
}

impl RodioAudioOutput {
  pub async fn new() -> Result<Self, RodioCreateError> {
    let (tx, rx) = mpsc::unbounded_channel();
    let (create_result_tx, create_result_rx) = oneshot::channel();
    let worker_thread = WorkerThread::new(create_result_tx, rx);
    create_result_rx.await.unwrap()?; // UNWRAP: errors if disconnected which only happens in panic -> we panic as well.
    let worker_thread = Arc::new(worker_thread);
    Ok(Self { tx, worker_thread })
  }
}

// Destruction

#[derive(Debug, Error)]
pub enum RodioDestroyError {
  #[error("Cannot destroy the Rodio audio output because one or more clones still exist. All clones must be dropped before stopping so that the worker thread can stop")]
  ClonesStillExist,
  #[error("Rodio audio output was destroyed, but the worker thread panicked before stopping with message: {0}")]
  ThreadPanicked(String),
  #[error("Rodio audio output was destroyed, but the worker thread panicked before stopping without a message")]
  ThreadPanickedSilently,
}

impl RodioAudioOutput {
  /// Destroys this audio output. Returns an error if it cannot be destroyed because there are still clones around, or
  /// if the worker thread was stopped but panicked.
  ///
  /// Dropping this audio output and all its clones will also properly destroy it, but ignores the panic produced by the
  /// worker thread (if any), and does not wait for the worker thread to complete first.
  pub fn destroy(self) -> Result<(), RodioDestroyError> {
    use RodioDestroyError::*;
    let RodioAudioOutput { tx, worker_thread } = self;
    drop(tx); // Dropping sender will cause the worker thread to break out of the loop and stop.
    let worker_thread = Arc::try_unwrap(worker_thread).map_err(|_| ClonesStillExist)?;
    if let Err(e) = worker_thread.join() { // Join does not block because worker thread stopped.
      return if let Some(msg) = try_panic_into_string(e) {
        Err(ThreadPanicked(msg))
      } else {
        Err(ThreadPanickedSilently)
      };
    };
    Ok(())
  }
}

// AudioOutput implementation

#[derive(Debug, Error)]
pub enum RodioSetAudioDataError {
  #[error("Failed to read audio data")]
  ReadFail(#[from] std::io::Error),
  #[error("Failed to create Rodio sink")]
  SinkCreateFail(#[from] rodio::PlayError),
  #[error("Failed to decode audio data")]
  DecodeFail(#[from] rodio::decoder::DecoderError),
  #[error("Failed to send command; worker thread was stopped")]
  SendCommandFail,
  #[error("Failed to receive command feedback; worker thread was stopped")]
  ReceiveCommandFeedbackFail,
}

#[derive(Debug, Error)]
pub enum RodioError {
  #[error("Failed to send command; worker thread was stopped")]
  SendCommandFail,
  #[error("Failed to receive command feedback; worker thread was stopped")]
  ReceiveCommandFeedbackFail,
}

#[async_trait]
impl AudioOutput for RodioAudioOutput {
  type SetAudioDataError = RodioSetAudioDataError;
  #[instrument(skip(self, data))]
  async fn set_audio_data(&self, _codec: Option<AudioCodec>, data: Vec<u8>) -> Result<(), RodioSetAudioDataError> {
    use RodioSetAudioDataError::*;
    let (tx, rx) = oneshot::channel();
    self.tx.send(Request::SetAudioData { data, tx }).map_err(|_| SendCommandFail)?;
    rx.await.map_err(|_| ReceiveCommandFeedbackFail)?
  }


  type IsPlayingError = RodioError;
  async fn is_playing(&self) -> Result<bool, Self::IsPlayingError> {
    self.send_receive(|tx| Request::IsPlaying { tx }).await
  }

  type PlayError = RodioError;
  async fn play(&self) -> Result<(), Self::PlayError> {
    self.send_receive(|tx| Request::Play { tx }).await
  }


  type IsPausedError = RodioError;
  async fn is_paused(&self) -> Result<bool, Self::IsPausedError> {
    self.send_receive(|tx| Request::IsPaused { tx }).await
  }

  type PauseError = RodioError;
  async fn pause(&self) -> Result<(), Self::PauseError> {
    self.send_receive(|tx| Request::Pause { tx }).await
  }


  type TogglePlayError = RodioError;
  async fn toggle_play(&self) -> Result<bool, Self::TogglePlayError> {
    self.send_receive(|tx| Request::TogglePlay { tx }).await
  }


  type IsStoppedError = RodioError;
  async fn is_stopped(&self) -> Result<bool, Self::IsStoppedError> {
    self.send_receive(|tx| Request::IsStopped { tx }).await
  }

  type StopError = RodioError;
  async fn stop(&self) -> Result<(), Self::StopError> {
    self.send_receive(|tx| Request::Stop { tx }).await
  }


  type GetVolumeError = RodioError;
  async fn get_volume(&self) -> Result<f64, Self::GetVolumeError> {
    self.send_receive(|tx| Request::GetVolume { tx }).await
  }

  type SetVolumeError = RodioError;
  async fn set_volume(&self, volume: f64) -> Result<(), Self::SetVolumeError> {
    self.send_receive(move |tx| Request::SetVolume { volume, tx }).await
  }
}

// Internals

impl RodioAudioOutput {
  async fn send_receive<T>(&self, request_fn: impl FnOnce(oneshot::Sender<T>) -> Request) -> Result<T, RodioError> {
    use RodioError::*;
    let (tx, rx) = oneshot::channel();
    self.tx.send(request_fn(tx)).map_err(|_| SendCommandFail)?;
    Ok(rx.await.map_err(|_| ReceiveCommandFeedbackFail)?)
  }
}

impl Debug for RodioAudioOutput {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("RodioAudioOutput")
      .finish()
  }
}

// Messages

enum Request {
  SetAudioData { data: Vec<u8>, tx: oneshot::Sender<Result<(), RodioSetAudioDataError>> },
  IsPlaying { tx: oneshot::Sender<bool> },
  Play { tx: oneshot::Sender<()> },
  IsPaused { tx: oneshot::Sender<bool> },
  Pause { tx: oneshot::Sender<()> },
  TogglePlay { tx: oneshot::Sender<bool> },
  IsStopped { tx: oneshot::Sender<bool> },
  Stop { tx: oneshot::Sender<()> },
  GetVolume { tx: oneshot::Sender<f64> },
  SetVolume { volume: f64, tx: oneshot::Sender<()> },
}

// Worker thread

struct WorkerThread {
  _stream: OutputStream,
  handle: OutputStreamHandle,
  sink: Option<Sink>,
  volume: f64,
  rx: mpsc::UnboundedReceiver<Request>,
}

impl WorkerThread {
  fn new(create_result_tx: oneshot::Sender<Result<(), RodioCreateError>>, rx: mpsc::UnboundedReceiver<Request>) -> JoinHandle<()> {
    thread::spawn(move || {
      let result: Result<_, RodioCreateError> = rodio::OutputStream::try_default()
        .map_err(|e| e.into());
      let (_stream, handle) = match result {
        Ok(v) => {
          // UNWRAP: errors if disconnected which only happens in panic -> we panic as well.
          create_result_tx.send(Ok(())).unwrap();
          v
        }
        Err(e) => {
          // UNWRAP: errors if disconnected which only happens in panic -> we panic as well.
          create_result_tx.send(Err(e)).unwrap();
          return;
        }
      };
      let worker_thread = WorkerThread {
        _stream,
        handle,
        sink: None,
        volume: 1.0,
        rx,
      };
      worker_thread.run();
    })
  }

  #[instrument(skip(self))]
  fn run(mut self) {
    while let Some(request) = self.rx.blocking_recv() { // Loop until all senders disconnect.
      // OK: in matches: receiver hung up -> we don't care.
      match request {
        Request::SetAudioData { data, tx } => tx.send(self.set_audio_data(data)).ok(),
        Request::IsPlaying { tx } => tx.send(self.is_playing()).ok(),
        Request::Play { tx } => tx.send(self.play()).ok(),
        Request::IsPaused { tx } => tx.send(self.is_paused()).ok(),
        Request::Pause { tx } => tx.send(self.pause()).ok(),
        Request::TogglePlay { tx } => tx.send(self.toggle_play()).ok(),
        Request::IsStopped { tx } => tx.send(self.is_stopped()).ok(),
        Request::Stop { tx } => tx.send(self.stop()).ok(),
        Request::GetVolume { tx } => tx.send(self.get_volume()).ok(),
        Request::SetVolume { volume, tx } => tx.send(self.set_volume(volume)).ok(),
      };
    }
  }

  #[instrument(skip(self, data))]
  fn set_audio_data(&mut self, data: Vec<u8>) -> Result<(), RodioSetAudioDataError> {
    if let Some(sink) = &self.sink { sink.stop(); }
    let sink = Sink::try_new(&self.handle)?;
    sink.set_volume(self.volume as f32);
    let cursor = Cursor::new(data);
    let decoder = rodio::decoder::Decoder::new(cursor)?;
    sink.append(decoder);
    self.sink = Some(sink);
    Ok(())
  }

  fn is_playing(&self) -> bool {
    !self.is_paused()
  }

  fn play(&self) {
    if let Some(sink) = &self.sink {
      sink.play();
    }
  }

  fn is_paused(&self) -> bool {
    self.sink.as_ref().map(|s| s.is_paused()).unwrap_or(false)
  }

  fn pause(&self) {
    if let Some(sink) = &self.sink {
      sink.pause();
    }
  }

  fn toggle_play(&self) -> bool {
    return if let Some(sink) = &self.sink {
      return if sink.is_paused() {
        sink.play();
        true
      } else {
        sink.pause();
        false
      };
    } else {
      false
    };
  }

  fn is_stopped(&self) -> bool {
    self.sink.is_none()
  }

  fn stop(&mut self) {
    if let Some(sink) = &self.sink {
      sink.stop();
    }
    self.sink = None;
  }

  fn get_volume(&self) -> f64 {
    self.volume
  }

  fn set_volume(&mut self, volume: f64) {
    self.volume = volume.clamp(0.0, 1.0);
    if let Some(sink) = &self.sink {
      sink.set_volume(volume as f32);
    }
  }
}
