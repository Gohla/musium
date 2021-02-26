use std::io::Cursor;
use std::sync::mpsc;
use std::thread;

use async_trait::async_trait;
use rodio::{OutputStream, OutputStreamHandle};
use thiserror::Error;

pub use musium_audio_output::AudioOutput;

pub struct RodioAudioOutput {
  thread_join_handle: thread::JoinHandle<()>,
  tx: mpsc::Sender<Command>,
}

// Creation

#[derive(Debug, Error)]
pub enum RodioCreateError {
  #[error("No default local audio output device was found")]
  NoDefaultOutputDevice,
}

impl RodioAudioOutput {
  pub fn new() -> Result<Self, RodioCreateError> {
    let (tx, rx) = mpsc::channel();
    let (create_result_tx, create_result_rx) = mpsc::channel();
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
    Ok(Self { tx, thread_join_handle })
  }

  pub fn stop(self) -> std::thread::Result<()> {
    let RodioAudioOutput { thread_join_handle, .. } = self;
    thread_join_handle.join()
  }
}

// Play

#[derive(Debug, Error)]
pub enum RodioPlayError {
  #[error(transparent)]
  ReadFail(#[from] std::io::Error),
  #[error(transparent)]
  PlayFail(#[from] rodio::PlayError),
  #[error("Failed to send the play command; thread handling the command was stopped")]
  CommandSendFail,
}

#[async_trait]
impl AudioOutput for RodioAudioOutput {
  type PlayError = RodioPlayError;
  async fn play(&mut self, audio_data: Vec<u8>, volume: f32) -> Result<(), Self::PlayError> {
    let tx = self.tx.clone();
    let (once_tx, once_rx) = futures::channel::oneshot::channel();
    tx.send(Command::Play { audio_data, volume, tx: once_tx }).map_err(|_| RodioPlayError::CommandSendFail)?;
    once_rx.await.unwrap() // CORRECTNESS: unwrap the result from awaiting, because it can never be cancelled.
  }
}

// Internals that run the message loop and perform commands.

enum Command {
  Play { audio_data: Vec<u8>, volume: f32, tx: futures::channel::oneshot::Sender<Result<(), RodioPlayError>> }
}

impl RodioAudioOutput {
  fn message_loop(_output_stream: OutputStream, output_stream_handle: OutputStreamHandle, rx: mpsc::Receiver<Command>) {
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
