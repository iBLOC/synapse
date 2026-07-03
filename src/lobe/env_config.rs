//! `LobeEnv` trait — env-driven config loader for lobe main.rs files.

pub trait LobeEnv: Sized {
    fn from_env() -> Result<Self, String>;
    fn validate(&self) -> Result<(), String> { Ok(()) }
}

pub fn load_lobe_env<T: LobeEnv>() -> Result<T, String> {
    let env = T::from_env()?;
    env.validate()?;
    Ok(env)
}
