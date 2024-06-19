use rusqlite::{Connection, Result as RusqliteResult};
use bitcoin::blockdata::block::{BlockHeader as BitcoinBlockHeader};
use bitcoin::hash_types::{BlockHash, TxMerkleNode, Txid};
use bitcoin::hashes::hex::FromHex;
use bitcoin::util::merkleblock::MerkleBlock;
use crate::block_header::BlockHeader;

pub fn get_block_header_by_height(conn: &Connection, height: u32) -> RusqliteResult<BlockHeader> {
    let mut stmt = conn.prepare("SELECT hash, height, previousblockhash, nextblockhash, merkleroot, time, bits, nonce FROM block_headers WHERE height = ?1")?;
    let block_header_iter = stmt.query_map([height], |row| {
        Ok(BlockHeader {
            hash: row.get(0)?,
            height: row.get(1)?,
            previousblockhash: row.get(2)?,
            nextblockhash: row.get(3)?,
            merkleroot: row.get(4)?,
            time: row.get(5)?,
            bits: row.get(6)?,
            nonce: row.get(7)?,
        })
    })?;

    for block_header in block_header_iter {
        return block_header;
    }
    Err(rusqlite::Error::QueryReturnedNoRows)
}

pub fn verify_transaction_spv(conn: &Connection, txid: &str, block_height: u32, merkle_block: &MerkleBlock) -> bool {
    if let Ok(block_header) = get_block_header_by_height(conn, block_height) {
        let header = BitcoinBlockHeader {
            version: 0, // version is not stored in our database
            prev_blockhash: BlockHash::from_hex(block_header.previousblockhash.as_deref().unwrap_or("0000000000000000000000000000000000000000000000000000000000000000")).unwrap(),
            merkle_root: TxMerkleNode::from_hex(&block_header.merkleroot).unwrap(),
            time: block_header.time as u32,
            bits: u32::from_str_radix(&block_header.bits, 16).unwrap(),
            nonce: block_header.nonce,
        };

        // Check if the block headers match
        if header != merkle_block.header {
            return false;
        }

        // Verify the Merkle proof manually
        let tx_hash = Txid::from_hex(txid).unwrap();
        let mut matched_tx_hashes = vec![];
        merkle_block.txn.extract_matches(&mut matched_tx_hashes, &mut vec![]).unwrap();
        
        matched_tx_hashes.iter().any(|hash| *hash == tx_hash)
    } else {
        false
    }
}

