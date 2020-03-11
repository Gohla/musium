use argon2::Config;
use rand::RngCore;
use thiserror::Error;

#[derive(Clone, Debug)]
pub struct PasswordHasher {
  secret_key: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum HashError {
  #[error(transparent)]
  HashFail(#[from] argon2::Error),
}

impl PasswordHasher {
  pub fn new<S: Into<Vec<u8>>>(secret_key: S) -> Self {
    let secret_key = secret_key.into();
    Self { secret_key }
  }

  pub fn hash<P: AsRef<[u8]>, S: AsRef<[u8]>>(&self, password: P, salt: S) -> Result<Vec<u8>, HashError> {
    Ok(argon2::hash_raw(password.as_ref(), salt.as_ref(), &self.config())?)
  }

  pub fn verify<P: AsRef<[u8]>, S: AsRef<[u8]>, H: AsRef<[u8]>>(&self, password: P, salt: S, hash: H) -> Result<bool, HashError> {
    Ok(argon2::verify_raw(password.as_ref(), salt.as_ref(), hash.as_ref(), &self.config())?)
  }

  const SALT_SIZE: usize = 32;

  pub fn generate_salt(&self) -> Vec<u8> {
    let mut salt = vec![0; Self::SALT_SIZE];
    let mut rng = rand::thread_rng();
    rng.fill_bytes(&mut salt);
    salt
  }

  fn config(&self) -> Config {
    // OPTO: prevent recreating the config all the time, without adding a lifetime to PasswordHasher, as Config takes
    // the secret key by reference, and Rust does not allow self-referential structs.
    Config {
      secret: &self.secret_key,
      ..Config::default()
    }
  }
}
