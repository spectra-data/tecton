// tecton-cli/src/config.rs
use figment::{
    Figment,
    providers::{Env, Format, Toml},
};
use std::path::PathBuf;
use tecton_core::config::CoreConfig;

pub fn load_config(config_path: Option<PathBuf>) -> anyhow::Result<CoreConfig> {
    let mut figment = Figment::new();

    if let Some(path) = config_path {
        figment = figment.merge(Toml::file(path));
    } else {
        figment = figment.merge(Toml::file("./config/config.toml"));
    }

    figment = figment.merge(Env::prefixed("TECTON_"));

    Ok(figment.extract()?)
}
