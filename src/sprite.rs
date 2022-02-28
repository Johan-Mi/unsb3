use crate::{
    deser::{Block, DeCtx},
    proc::Proc,
};
use serde::{de::Error, Deserialize, Deserializer};
use std::{cell::Cell, collections::HashMap};

#[derive(Debug, Deserialize)]
pub(crate) struct Sprite {
    #[serde(rename = "blocks")]
    #[serde(deserialize_with = "deserialize_blocks")]
    pub procs: Vec<Proc>,
    #[serde(default)]
    pub x: Cell<f64>,
    #[serde(default)]
    pub y: Cell<f64>,
}

fn deserialize_blocks<'de, D>(deserializer: D) -> Result<Vec<Proc>, D::Error>
where
    D: Deserializer<'de>,
{
    let blocks = <HashMap<String, Block>>::deserialize(deserializer)?;
    let ctx = DeCtx::new(blocks);
    ctx.build_procs().map_err(D::Error::custom)
}
