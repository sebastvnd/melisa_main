use serde::Deserialize;
use std::fs;
use std::io::Write;
use tempfile::NamedTempFile;

use once_cell::sync::Lazy;

pub static CONFIG: Lazy<Config> = Lazy::new(|| Config::from_file("config.toml").unwrap());

// ex: 
// let config = Config::from_file(
//     file.path().to_str().unwrap())
// .unwrap();

#[derive(Debug, Deserialize)]
pub struct Config {
    pub host: String,
    pub port: u16,
}

impl Config {
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config = toml::from_str(&content)?;
        Ok(config)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_from_file() {
        let mut file = NamedTempFile::new().unwrap();

        write!(
            file,
            r#"
    host = "127.0.0.1"
    port = 8080
    "#
        )
        .unwrap();

        let config = Config::from_file(file.path().to_str().unwrap()).unwrap();

        assert_eq!(config.port, 8080);
    }
}
