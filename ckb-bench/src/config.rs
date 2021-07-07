use serde_derive::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Deserialize, Serialize, Debug, Clone, Eq, Ord, PartialOrd, PartialEq)]
pub struct Url(#[serde(with = "url_serde")] pub url::Url);

impl Deref for Url {
    type Target = url::Url;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for Url {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for Url {
    type Err = url::ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Url::parse(s)
    }
}

impl Url {
    pub fn parse(input: &str) -> Result<Url, url::ParseError> {
        let url = url::Url::parse(input)?;
        Ok(Url(url))
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Spec {
    pub working_dir: PathBuf,
    pub miner: Option<MinerConfig>,
    pub users: Vec<String>,

    pub benchmarks: Vec<BenchmarkConfig>,

    // TODO consensus_cellbase_maturity
    pub consensus_cellbase_maturity: u64,
    pub confirmation_blocks: u64,
    pub ensure_matured_capacity_greater_than: u64,

    pub method_to_eval_network_stable: MethodToEvalNetStable,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct MinerConfig {
    pub privkey: String,
    pub block_time: u64,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct BenchmarkConfig {
    pub transaction_type: TransactionType,
    pub send_delay: u64, // micros
    pub method_to_eval_net_stable: Option<MethodToEvalNetStable>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub enum MethodToEvalNetStable {
    #[allow(dead_code)]
    RecentBlocktxnsNearly { window: u64, margin: u64 },
    #[allow(dead_code)]
    CustomBlocksElapsed { warmup: u64, window: u64 },
    #[allow(dead_code)]
    Never,
    #[allow(dead_code)]
    TimedTask { duration_time: u64 },
}

impl Spec {
    pub fn load<P: AsRef<Path>>(filepath: &P) -> Self {
        let content = fs::read_to_string(filepath).unwrap_or_else(|err| {
            crate::prompt_and_exit!(
                "failed to read \"{}\", error: {}",
                filepath.as_ref().to_string_lossy(),
                err
            )
        });
        let spec: Spec = toml::from_str(&content).unwrap_or_else(|err| {
            crate::prompt_and_exit!(
                "failed to deserialize toml file \"{}\", error: {}",
                filepath.as_ref().to_string_lossy(),
                err
            )
        });
        spec
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub enum TransactionType {
    In1Out1,
    In2Out2,
    In3Out3,
}