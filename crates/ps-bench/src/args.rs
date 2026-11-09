//! Minimal flag parser (no clap dependency): --key value plus bare --flag.

pub struct Args {
    raw: Vec<String>,
}

impl Args {
    pub fn parse(tokens: &[String]) -> Self {
        Self { raw: tokens.to_vec() }
    }

    pub fn has(&self, key: &str) -> bool {
        self.raw.iter().any(|t| t == key)
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        let mut i = 0;
        while i + 1 < self.raw.len() {
            if self.raw[i] == key {
                return Some(self.raw[i + 1].as_str());
            }
            i += 1;
        }
        None
    }

    pub fn required(&self, key: &str) -> anyhow::Result<&str> {
        self.get(key)
            .ok_or_else(|| anyhow::anyhow!("missing required option {key}"))
    }

    pub fn u64_or(&self, key: &str, default: u64) -> anyhow::Result<u64> {
        match self.get(key) {
            Some(v) => Ok(v.parse::<u64>()?),
            None => Ok(default),
        }
    }

    pub fn usize_or(&self, key: &str, default: usize) -> anyhow::Result<usize> {
        Ok(self.u64_or(key, default as u64)? as usize)
    }

    pub fn f64_or(&self, key: &str, default: f64) -> anyhow::Result<f64> {
        match self.get(key) {
            Some(v) => Ok(v.parse::<f64>()?),
            None => Ok(default),
        }
    }
}
