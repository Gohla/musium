use std::io::Cursor;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use kira::{
  instance::{
    handle::InstanceHandle,
    InstanceSettings,
    InstanceState,
    PauseInstanceSettings,
    ResumeInstanceSettings,
    StopInstanceSettings,
  },
  manager::{
    AudioManager, AudioManagerSettings,
  },
  sound::{
    handle::SoundHandle,
    Sound,
    SoundSettings,
  },
  Value,
};
use thiserror::Error;

use musium_audio_output::{AudioCodec, AudioOutput};

#[derive(Clone)]
pub struct KiraAudioOutput {
  inner: Arc<Mutex<Inner>>,
}

// Creation

#[derive(Debug, Error)]
pub enum KiraCreateError {
  #[error("Failed to create Kira audio manager")]
  AudioManagerCreateFail(#[from] kira::manager::error::SetupError),
}

impl KiraAudioOutput {
  pub fn new() -> Result<Self, KiraCreateError> {
    let audio_manager = AudioManager::new(AudioManagerSettings::default())?;
    let inner = Arc::new(Mutex::new(Inner {
      audio_manager,
      current_sound_handle: None,
      current_instance_handle: None,
      current_volume: 0.1,
    }));
    Ok(Self { inner })
  }
}

// AudioOutput implementation

#[derive(Debug, Error)]
pub enum KiraSetAudioDataError {
  #[error("Failed to load sound")]
  LoadSoundFail(#[from] kira::sound::error::SoundFromFileError),
  #[error("Failed to add sound to audio manager")]
  AddSoundFail(#[from] kira::manager::error::AddSoundError),
}

#[async_trait]
impl AudioOutput for KiraAudioOutput {
  type SetAudioDataError = KiraSetAudioDataError;
  async fn set_audio_data(&self, codec: AudioCodec, data: Vec<u8>) -> Result<(), Self::SetAudioDataError> {
    use AudioCodec::*;
    let cursor = Cursor::new(data);
    let sound = match codec {
      Mp3 => { Sound::from_mp3_reader(cursor, SoundSettings::default()) }
      Ogg => { Sound::from_ogg_reader(cursor, SoundSettings::default()) }
      Flac => { Sound::from_flac_reader(cursor, SoundSettings::default()) }
      Wav => { Sound::from_wav_reader(cursor, SoundSettings::default()) }
    }?;
    let mut inner = self.inner.lock().unwrap();
    inner.current_sound_handle = Some(inner.audio_manager.add_sound(sound)?);
    Ok(())
  }


  type IsPlayingError = ();
  async fn is_playing(&self) -> Result<bool, Self::IsPlayingError> {
    let mut inner = self.inner.lock().unwrap();
    let result = if let Some(instance_handle) = &inner.current_instance_handle {
      instance_handle.state() == InstanceState::Playing
    } else {
      false
    };
    Ok(result)
  }

  type PlayError = kira::CommandError;
  async fn play(&self) -> Result<(), Self::PlayError> {
    let mut inner = self.inner.lock().unwrap();
    if let Some(instance_handle) = &mut inner.current_instance_handle {
      instance_handle.resume(ResumeInstanceSettings::default())?;
      return Ok(());
    }
    if let Some(mut sound_handle) = &inner.current_sound_handle {
      let instance_handle = sound_handle.play(InstanceSettings {
        volume: Value::Fixed(inner.current_volume),
        ..InstanceSettings::default()
      })?;
      inner.current_instance_handle = Some(instance_handle);
    }
    Ok(())
  }


  type IsPausedError = ();
  async fn is_paused(&self) -> Result<bool, Self::IsPausedError> {
    let mut inner = self.inner.lock().unwrap();
    let result = if let Some(instance_handle) = &inner.current_instance_handle {
      match instance_handle.state() {
        InstanceState::Paused(_) => true,
        InstanceState::Pausing(_) => true,
        _ => false
      }
    } else {
      false
    };
    Ok(result)
  }

  type PauseError = kira::CommandError;
  async fn pause(&self) -> Result<(), Self::PauseError> {
    let mut inner = self.inner.lock().unwrap();
    if let Some(instance_handle) = &mut inner.current_instance_handle {
      instance_handle.pause(PauseInstanceSettings::default())?;
    }
    Ok(())
  }


  type TogglePlayError = kira::CommandError;
  async fn toggle_play(&self) -> Result<bool, Self::TogglePlayError> {
    let mut inner = self.inner.lock().unwrap();
    if let Some(instance_handle) = &mut inner.current_instance_handle {
      let result = match instance_handle.state() {
        InstanceState::Playing => {
          instance_handle.pause(PauseInstanceSettings::default())?;
          true
        }
        InstanceState::Paused(_) => {
          instance_handle.resume(ResumeInstanceSettings::default())?;
          true
        }
        InstanceState::Pausing(_) => {
          instance_handle.resume(ResumeInstanceSettings::default())?;
          true
        }
        _ => false
      };
      Ok(result)
    } else {
      Ok(false)
    }
  }


  type IsStoppedError = ();
  async fn is_stopped(&self) -> Result<bool, Self::IsStoppedError> {
    let mut inner = self.inner.lock().unwrap();
    let result = if let Some(instance_handle) = &inner.current_instance_handle {
      match instance_handle.state() {
        InstanceState::Stopped => true,
        InstanceState::Stopping => true,
        _ => false
      }
    } else {
      false
    };
    Ok(result)
  }

  type StopError = kira::CommandError;
  async fn stop(&self) -> Result<(), Self::StopError> {
    let mut inner = self.inner.lock().unwrap();
    if let Some(instance_handle) = &mut inner.current_instance_handle {
      instance_handle.stop(StopInstanceSettings::default())?;
    }
    Ok(())
  }


  type GetVolumeError = ();
  async fn get_volume(&self) -> Result<f64, Self::GetVolumeError> {
    let mut inner = self.inner.lock().unwrap();
    Ok(inner.current_volume)
  }

  type SetVolumeError = kira::CommandError;
  async fn set_volume(&self, volume: f64) -> Result<(), Self::SetVolumeError> {
    let mut inner = self.inner.lock().unwrap();
    if let Some(instance_handle) = &mut inner.current_instance_handle {
      instance_handle.set_volume(Value::Fixed(volume))?;
    }
    inner.current_volume = volume;
    Ok(())
  }
}

// Internals

struct Inner {
  audio_manager: AudioManager,
  current_sound_handle: Option<SoundHandle>,
  current_instance_handle: Option<InstanceHandle>,
  current_volume: f64,
}
