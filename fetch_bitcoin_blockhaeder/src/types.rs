use serde::Deserialize;
#[derive(Deserialize)]
pub struct GetBlockHashResponse {
    pub result: String,
    #[allow(dead_code)]
    pub error: Option<serde_json::Value>,
    #[allow(dead_code)]
    pub id: String,
}

#[derive(Deserialize)]
pub struct GetBlockHeaderResponse {
    pub result: BlockHeader,
    #[allow(dead_code)]
    pub error: Option<serde_json::Value>,
    #[allow(dead_code)]
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct BlockHeader {
    pub hash: String,
    pub height: u64,
    pub version: i32,
    pub previousblockhash: Option<String>,
    pub nextblockhash: Option<String>,
    pub merkleroot: String,
    pub time: u64,
    pub bits: String,
    pub nonce: u32,
    pub difficulty: f64,
    pub chainwork: String,  
}
