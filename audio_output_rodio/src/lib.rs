use std::io::Cursor;
use std::sync::Arc;
use std::thread;

use async_trait::async_trait;
use futures::channel::oneshot;
use rodio::{OutputStream, OutputStreamHandle};
use thiserror::Error;

pub use musium_audio_output::AudioOutput;

#[derive(Clone)]
pub struct RodioAudioOutput {
  inner: Arc<Inner>,
}

struct Inner {
  thread_join_handle: thread::JoinHandle<()>,
  tx: crossbeam_channel::Sender<Command>,
}

// Creation

#[derive(Debug, Error)]
pub enum RodioCreateError {
  #[error("No default local audio output device was found")]
  NoDefaultOutputDevice,
}

impl RodioAudioOutput {
  pub fn new() -> Result<Self, RodioCreateError> {
    let (tx, rx) = crossbeam_channel::unbounded();
    let (create_result_tx, create_result_rx) = crossbeam_channel::bounded(0);
    let thread_join_handle = thread::spawn(move || {
      let (output_stream, output_stream_handle) = match rodio::OutputStream::try_default().map_err(|_| RodioCreateError::NoDefaultOutputDevice) {
        Ok(v) => {
          create_result_tx.send(Ok(())).unwrap();
          v
        }
        Err(e) => {
          create_result_tx.send(Err(e)).unwrap();
          return;
        }
      };
      Self::message_loop(output_stream, output_stream_handle, rx)
    });
    create_result_rx.recv().unwrap()?;
    let inner = Arc::new(Inner { tx, thread_join_handle });
    Ok(Self { inner })
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
  pub fn stop(self) -> Result<(), RodioDestroyError> {
    use RodioDestroyError::*;
    let Inner { thread_join_handle, .. } = Arc::try_unwrap(self.inner).map_err(|_| ClonesStillExist)?;
    // Because we did not match on Inner.tx, it is dropped and the thread will stop.
    match thread_join_handle.join() {
      Err(e) => if let Some(msg) = e.downcast_ref::<&'static str>() {
        Err(ThreadPanicked(msg.to_string()))
      } else if let Some(msg) = e.downcast_ref::<String>() {
        Err(ThreadPanicked(msg.to_string()))
      } else {
        Err(ThreadPanickedSilently)
      }
      Ok(_) => Ok(()),
    }
  }
}

// Play

#[derive(Debug, Error)]
pub enum RodioPlayError {
  #[error(transparent)]
  ReadFail(#[from] std::io::Error),
  #[error(transparent)]
  PlayFail(#[from] rodio::PlayError),
  #[error("Failed to send command; worker thread was stopped")]
  SendCommandFail,
  #[error("Failed to receive command feedback; worker thread was stopped")]
  ReceiveCommandFeedbackFail,
}

#[async_trait]
impl AudioOutput for RodioAudioOutput {
  type PlayError = RodioPlayError;
  async fn play(&self, audio_data: Vec<u8>, volume: f32) -> Result<(), RodioPlayError> {
    use RodioPlayError::*;
    let (tx, rx) = oneshot::channel();
    self.inner.tx.send(Command::Play { audio_data, volume, tx }).map_err(|_| SendCommandFail)?;
    rx.await.map_err(|_| ReceiveCommandFeedbackFail)?
  }
}

// Internals that run the message loop and perform commands.

enum Command {
  Play { audio_data: Vec<u8>, volume: f32, tx: oneshot::Sender<Result<(), RodioPlayError>> }
}

impl RodioAudioOutput {
  fn message_loop(_output_stream: OutputStream, output_stream_handle: OutputStreamHandle, rx: crossbeam_channel::Receiver<Command>) {
    loop {
      match rx.recv() {
        Ok(message) => match message {
          Command::Play { audio_data, volume, tx } => tx.send(Self::play(audio_data, volume, &output_stream_handle)).ok(),
        }
        Err(_) => break, // Sender has disconnected, stop the loop.
      };
    }
  }

  fn play(audio_data: Vec<u8>, volume: f32, stream_handle: &OutputStreamHandle) -> Result<(), RodioPlayError> {
    let cursor = Cursor::new(audio_data);
    let sink = stream_handle.play_once(cursor)?;
    sink.set_volume(volume);
    Ok(())
  }
}
