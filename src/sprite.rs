use crate::{
    deser::{Block, DeCtx},
    proc::{BunchOfProcs, Custom, Proc},
};
use serde::{de::Error, Deserialize, Deserializer};
use std::{cell::Cell, collections::HashMap};

#[derive(Debug)]
pub struct Sprite {
    pub procs: Vec<Proc>,
    pub custom_procs: HashMap<String, Custom>,
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
        blocks: (Vec<Proc>, HashMap<String, Custom>),
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
                    procs: blocks.0,
                    custom_procs: blocks.1,
                    x: Cell::new(x),
                    y: Cell::new(y),
                },
            )
        })
        .collect())
}

fn deserialize_blocks<'de, D>(deserializer: D) -> Result<BunchOfProcs, D::Error>
where
    D: Deserializer<'de>,
{
    let blocks = <HashMap<String, Block>>::deserialize(deserializer)?;
    let ctx = DeCtx::new(blocks);
    ctx.build_procs().map_err(D::Error::custom)
}
