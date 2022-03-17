use crate::{
    deser::{Block, DeCtx},
    proc::Procs,
};
use serde::{de::Error, Deserialize, Deserializer};
use std::{cell::Cell, collections::HashMap};

#[derive(Debug)]
pub struct Sprite {
    pub procs: Procs,
    pub x: Cell<f64>,
    pub y: Cell<f64>,
}

pub fn deserialize_sprites<'de, D>(
    deserializer: D,
) -> Result<HashMap<String, Sprite>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct DeSprite {
        name: String,
        #[serde(deserialize_with = "deserialize_blocks")]
        blocks: Procs,
        #[serde(default)]
        x: f64,
        #[serde(default)]
        y: f64,
    }

    let sprites = <Vec<DeSprite>>::deserialize(deserializer)?;

    Ok(sprites
        .into_iter()
        .map(|DeSprite { name, blocks, x, y }| {
            (
                name,
                Sprite {
                    procs: blocks,
                    x: Cell::new(x),
                    y: Cell::new(y),
                },
            )
        })
        .collect())
}

fn deserialize_blocks<'de, D>(deserializer: D) -> Result<Procs, D::Error>
where
    D: Deserializer<'de>,
{
    let blocks = <HashMap<String, Block>>::deserialize(deserializer)?;
    let ctx = DeCtx::new(blocks);
    ctx.build_procs().map_err(D::Error::custom)
}
