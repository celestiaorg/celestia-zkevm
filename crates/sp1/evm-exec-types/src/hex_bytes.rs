use hex::{encode_upper, FromHex};
use serde::Deserialize;
use serde::{self, Deserializer, Serializer};

pub fn serialize<S>(bytes: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let hex_str = encode_upper(bytes);
    serializer.serialize_str(&hex_str)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let bytes = <[u8; 32]>::from_hex(&s).map_err(serde::de::Error::custom)?;
    Ok(bytes)
}
