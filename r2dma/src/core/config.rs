use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
pub struct Config {
    pub buffer_size: usize,
    pub buffer_count: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            buffer_size: 1 << 20,
            buffer_count: 64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config() {
        let ser = Config::default();
        let str = toml::to_string_pretty(&ser).unwrap();
        let des: Config = toml::from_str(&str).unwrap();
        assert_eq!(ser, des);
    }
}
