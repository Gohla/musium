use std::io::Cursor;
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;

use async_trait::async_trait;
use futures::channel::oneshot;
use rodio::{OutputStream, OutputStreamHandle, Sink};
use thiserror::Error;
use tracing::instrument;

pub use musium_audio_output::AudioOutput;
use musium_core::panic::try_panic_into_string;

#[derive(Clone)]
pub struct RodioAudioOutput {
  tx: crossbeam_channel::Sender<Request>,
  worker_thread: Arc<thread::JoinHandle<()>>,
}

// Creation

#[derive(Debug, Error)]
pub enum RodioCreateError {
  #[error("Failed to create Rodio stream")]
  StreamCreateFail(#[from] rodio::StreamError),
}

impl RodioAudioOutput {
  pub fn new() -> Result<Self, RodioCreateError> {
    let (tx, rx) = crossbeam_channel::unbounded();
    let (create_result_tx, create_result_rx) = crossbeam_channel::bounded(0);
    let worker_thread = WorkerThread::new(create_result_tx, rx);
    create_result_rx.recv().unwrap()?; // UNWRAP: errors if disconnected which only happens in panic -> we panic as well.
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

// Play

#[derive(Debug, Error)]
pub enum RodioPlayError {
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

#[async_trait]
impl AudioOutput for RodioAudioOutput {
  type PlayError = RodioPlayError;
  #[instrument(skip(self, audio_data))]
  async fn play(&self, audio_data: Vec<u8>, volume: f32) -> Result<(), RodioPlayError> {
    use RodioPlayError::*;
    let (tx, rx) = oneshot::channel();
    self.tx.send(Request::Play { audio_data, volume, tx }).map_err(|_| SendCommandFail)?;
    rx.await.map_err(|_| ReceiveCommandFeedbackFail)?
  }
}

// Internals

// Messages

enum Request {
  Play { audio_data: Vec<u8>, volume: f32, tx: oneshot::Sender<Result<(), RodioPlayError>> }
}

// Worker thread

struct WorkerThread {
  _stream: OutputStream,
  handle: OutputStreamHandle,
  current_sink: Option<Sink>,
  rx: crossbeam_channel::Receiver<Request>,
}

impl WorkerThread {
  fn new(create_result_tx: crossbeam_channel::Sender<Result<(), RodioCreateError>>, rx: crossbeam_channel::Receiver<Request>) -> JoinHandle<()> {
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
      let worker_thread = WorkerThread { _stream, handle, current_sink: None, rx };
      worker_thread.run();
    })
  }

  #[instrument(skip(self))]
  fn run(mut self) {
    while let Ok(request) = self.rx.recv() { // Loop until all senders disconnect.
      match request {
        Request::Play { audio_data, volume, tx } => {
          tx.send(self.play(audio_data, volume)).ok(); // OK: receiver hung up -> we don't care.
        }
      };
    }
  }

  #[instrument(skip(self, audio_data))]
  fn play(&mut self, audio_data: Vec<u8>, volume: f32) -> Result<(), RodioPlayError> {
    if let Some(sink) = &self.current_sink {
      sink.stop();
    }
    let sink = Sink::try_new(&self.handle)?;
    sink.set_volume(volume);
    let cursor = Cursor::new(audio_data);
    let decoder = rodio::decoder::Decoder::new(cursor)?;
    sink.append(decoder);
    self.current_sink = Some(sink);
    Ok(())
  }
}
