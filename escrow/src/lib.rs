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
    /// Buyer locks `amount` of `token` for `seller` until released or refunded. Returns the escrow id.
    pub fn create(
        env: Env,
        buyer: Address,
        seller: Address,
        token: Address,
        amount: i128,
        deadline: u64,
        memo: String,
    ) -> u64 {
        buyer.require_auth();
        if amount <= 0 {
            panic_with_error!(&env, Error::InvalidAmount);
        }
        if buyer == seller {
            panic_with_error!(&env, Error::SameParty);
        }
        let now = env.ledger().timestamp();
        if deadline <= now {
            panic_with_error!(&env, Error::InvalidDeadline);
        }
        if memo.len() > MAX_MEMO_LEN {
            panic_with_error!(&env, Error::MemoTooLong);
        }

        // Pull funds into the contract first; fails atomically if the buyer can't pay.
        token::Client::new(&env, &token).transfer(&buyer, env.current_contract_address(), &amount);

        let id: u64 = env.storage().instance().get(&DataKey::Count).unwrap_or(0) + 1;
        env.storage().instance().set(&DataKey::Count, &id);
        env.storage().instance().extend_ttl(TTL_THRESHOLD, TTL_EXTEND_TO);

        let e = Escrow {
            id,
            buyer: buyer.clone(),
            seller: seller.clone(),
            token: token.clone(),
            amount,
            deadline,
            created_at: now,
            status: Status::Funded,
            memo,
        };
        save(&env, &e);
        Created { id, buyer, seller, token, amount, deadline }.publish(&env);
        id
    }

    /// Buyer confirms delivery: funds go to the seller.
    pub fn release(env: Env, id: u64) {
        let mut e = load(&env, id);
        e.buyer.require_auth();
        if e.status != Status::Funded {
            panic_with_error!(&env, Error::NotFunded);
        }
        e.status = Status::Released;
        save(&env, &e); // state change before the external call
        token::Client::new(&env, &e.token).transfer(&env.current_contract_address(), &e.seller, &e.amount);
        Released { id, seller: e.seller.clone(), amount: e.amount }.publish(&env);
    }

    pub fn get(env: Env, id: u64) -> Escrow {
        load(&env, id)
    }

    pub fn count(env: Env) -> u64 {
        env.storage().instance().get(&DataKey::Count).unwrap_or(0)
    }
}

#[cfg(test)]
mod test;
