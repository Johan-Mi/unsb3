use crate::proc::Proc;
use serde::{Deserialize, Deserializer};
use serde_json::Value as Json;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub(crate) struct Sprite {
    #[serde(rename = "blocks")]
    #[serde(deserialize_with = "deserialize_blocks")]
    pub procs: Vec<Proc>,
}

fn deserialize_blocks<'de, D>(deserializer: D) -> Result<Vec<Proc>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct Block {
        opcode: String,
        parent: Option<String>,
        next: Option<String>,
        inputs: HashMap<String, Json>,
        fields: HashMap<String, Json>,
    }

    let blocks = <HashMap<String, Block>>::deserialize(deserializer)?;

    todo!()
}
