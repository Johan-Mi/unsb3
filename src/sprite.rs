use crate::{
    deser::{Block, DeCtx},
    proc::Procs,
};
use serde::{de::Error, Deserialize, Deserializer};
use smol_str::SmolStr;
use std::{cell::Cell, collections::HashMap};

#[derive(Debug)]
pub struct Sprite {
    pub procs: Procs,
    pub x: Cell<f64>,
    pub y: Cell<f64>,
}

pub fn deserialize_sprites<'de, D>(
    deserializer: D,
) -> Result<HashMap<SmolStr, Sprite>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct DeSprite<'a> {
        name: SmolStr,
        #[serde(borrow)]
        blocks: HashMap<SmolStr, Block<'a>>,
        #[serde(default)]
        x: f64,
        #[serde(default)]
        y: f64,
    }

    let sprites = <Vec<DeSprite>>::deserialize(deserializer)?;

    sprites
        .into_iter()
        .map(|DeSprite { name, blocks, x, y }| {
            let ctx = DeCtx::new(blocks);
            let procs = ctx.build_procs().map_err(D::Error::custom)?;
            Ok((
                name,
                Sprite {
                    procs,
                    x: Cell::new(x),
                    y: Cell::new(y),
                },
            ))
        })
        .collect()
}
