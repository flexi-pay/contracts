//! Starling Escrow — a Soroban smart contract.
//!
//! A buyer locks any Stellar token (XLM, USDC, … via their Stellar Asset Contract)
//! for a seller. The buyer releases the funds once they receive what they paid for.
//! If something goes wrong:
//!   * the seller can refund the buyer at any time, and
//!   * the buyer can reclaim the funds themselves once the deadline has passed.
//!
//! Nobody else — including whoever deployed the contract — can move escrowed funds.
#![no_std]

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, panic_with_error, token,
    Address, Env, String, Vec,
};

/// ~30 days of ledgers (5s each). Escrow records are kept alive at least this long after each touch.
const TTL_EXTEND_TO: u32 = 518_400;
const TTL_THRESHOLD: u32 = TTL_EXTEND_TO - 17_280; // extend when < ~29 days remain
const MAX_LIST: u32 = 50;
const MAX_MEMO_LEN: u32 = 64;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NotFound = 1,
    InvalidAmount = 2,
    InvalidDeadline = 3,
    NotFunded = 4,
    NotAllowed = 5,
    TooEarly = 6,
    SameParty = 7,
    MemoTooLong = 8,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    Funded,
    Released,
    Refunded,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Escrow {
    pub id: u64,
    pub buyer: Address,
    pub seller: Address,
    pub token: Address,
    pub amount: i128,
    /// Unix timestamp (seconds) after which the buyer may refund themselves.
    pub deadline: u64,
    pub created_at: u64,
    pub status: Status,
    pub memo: String,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Count,
    Escrow(u64),
}

#[contractevent(topics = ["escrow", "created"])]
pub struct Created {
    #[topic]
    pub id: u64,
    pub buyer: Address,
    pub seller: Address,
    pub token: Address,
    pub amount: i128,
    pub deadline: u64,
}

#[contractevent(topics = ["escrow", "released"])]
pub struct Released {
    #[topic]
    pub id: u64,
    pub seller: Address,
    pub amount: i128,
}

#[contractevent(topics = ["escrow", "refunded"])]
pub struct Refunded {
    #[topic]
    pub id: u64,
    pub buyer: Address,
    pub amount: i128,
    pub by: Address,
}

#[contract]
pub struct EscrowContract;

fn load(env: &Env, id: u64) -> Escrow {
    let key = DataKey::Escrow(id);
    let e: Escrow = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic_with_error!(env, Error::NotFound));
    env.storage().persistent().extend_ttl(&key, TTL_THRESHOLD, TTL_EXTEND_TO);
    e
}

fn save(env: &Env, e: &Escrow) {
    let key = DataKey::Escrow(e.id);
    env.storage().persistent().set(&key, e);
    env.storage().persistent().extend_ttl(&key, TTL_THRESHOLD, TTL_EXTEND_TO);
}

#[contractimpl]
impl EscrowContract {
    pub fn get(env: Env, id: u64) -> Escrow {
        load(&env, id)
    }

    pub fn count(env: Env) -> u64 {
        env.storage().instance().get(&DataKey::Count).unwrap_or(0)
    }
}
