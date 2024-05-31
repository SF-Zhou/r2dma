use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
pub struct Config {
    pub buffer_size: usize,
    pub buffer_count: usize,
    pub max_cqe: usize,
    pub max_wr: usize,
    pub max_sge: usize,
    pub work_pool_size: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            buffer_size: 1 << 20,
            buffer_count: 64,
            max_cqe: 64,
            max_wr: 10,
            max_sge: 5,
            work_pool_size: 1024,
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
