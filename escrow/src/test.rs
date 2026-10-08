#![cfg(test)]
extern crate std;

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token::{StellarAssetClient, TokenClient},
    Address, Env, String,
};

struct Setup<'a> {
    env: Env,
    client: EscrowContractClient<'a>,
    token: TokenClient<'a>,
    buyer: Address,
    seller: Address,
}

fn setup() -> Setup<'static> {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|l| l.timestamp = 1_000);

    let admin = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(admin);
    let token = TokenClient::new(&env, &sac.address());
    let buyer = Address::generate(&env);
    let seller = Address::generate(&env);
    StellarAssetClient::new(&env, &sac.address()).mint(&buyer, &1_000);

    let id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &id);
    Setup { env, client, token, buyer, seller }
}

fn err(e: Error) -> soroban_sdk::Error {
    soroban_sdk::Error::from_contract_error(e as u32)
}

fn memo(env: &Env, s: &str) -> String {
    String::from_str(env, s)
}

#[test]
fn create_locks_funds_and_release_pays_seller() {
    let s = setup();
    let id = s.client.create(&s.buyer, &s.seller, &s.token.address, &400, &2_000, &memo(&s.env, "Phone"));
    assert_eq!(id, 1);
    assert_eq!(s.token.balance(&s.buyer), 600);
    assert_eq!(s.token.balance(&s.client.address), 400);

    let e = s.client.get(&id);
    assert_eq!(e.status, Status::Funded);
    assert_eq!(e.amount, 400);
    assert_eq!(e.created_at, 1_000);

    s.client.release(&id);
    assert_eq!(s.token.balance(&s.seller), 400);
    assert_eq!(s.token.balance(&s.client.address), 0);
    assert_eq!(s.client.get(&id).status, Status::Released);
}

#[test]
fn release_requires_buyer_auth() {
    let s = setup();
    let id = s.client.create(&s.buyer, &s.seller, &s.token.address, &100, &2_000, &memo(&s.env, ""));
    s.client.release(&id);
    let auths = s.env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, s.buyer);
}

#[test]
fn seller_can_refund_any_time() {
    let s = setup();
    let id = s.client.create(&s.buyer, &s.seller, &s.token.address, &250, &5_000, &memo(&s.env, ""));
    s.client.refund(&id, &s.seller);
    assert_eq!(s.token.balance(&s.buyer), 1_000);
    assert_eq!(s.client.get(&id).status, Status::Refunded);
}

#[test]
fn buyer_refund_only_after_deadline() {
    let s = setup();
    let id = s.client.create(&s.buyer, &s.seller, &s.token.address, &250, &5_000, &memo(&s.env, ""));
    assert_eq!(s.client.try_refund(&id, &s.buyer), Err(Ok(err(Error::TooEarly))));
    s.env.ledger().with_mut(|l| l.timestamp = 5_000);
    s.client.refund(&id, &s.buyer);
    assert_eq!(s.token.balance(&s.buyer), 1_000);
}

#[test]
fn strangers_cannot_refund() {
    let s = setup();
    let id = s.client.create(&s.buyer, &s.seller, &s.token.address, &250, &5_000, &memo(&s.env, ""));
    let stranger = Address::generate(&s.env);
    s.env.ledger().with_mut(|l| l.timestamp = 9_999);
    assert_eq!(s.client.try_refund(&id, &stranger), Err(Ok(err(Error::NotAllowed))));
}

#[test]
fn cannot_settle_twice() {
    let s = setup();
    let id = s.client.create(&s.buyer, &s.seller, &s.token.address, &100, &2_000, &memo(&s.env, ""));
    s.client.release(&id);
    assert_eq!(s.client.try_release(&id), Err(Ok(err(Error::NotFunded))));
    assert_eq!(s.client.try_refund(&id, &s.seller), Err(Ok(err(Error::NotFunded))));
    assert_eq!(s.token.balance(&s.seller), 100);
}

#[test]
fn validates_inputs() {
    let s = setup();
    let t = &s.token.address;
    let m = memo(&s.env, "");
    assert_eq!(s.client.try_create(&s.buyer, &s.seller, t, &0, &2_000, &m), Err(Ok(err(Error::InvalidAmount))));
    assert_eq!(s.client.try_create(&s.buyer, &s.buyer, t, &10, &2_000, &m), Err(Ok(err(Error::SameParty))));
    assert_eq!(s.client.try_create(&s.buyer, &s.seller, t, &10, &1_000, &m), Err(Ok(err(Error::InvalidDeadline))));
    let long = memo(&s.env, "x".repeat(65).as_str());
    assert_eq!(s.client.try_create(&s.buyer, &s.seller, t, &10, &2_000, &long), Err(Ok(err(Error::MemoTooLong))));
    assert_eq!(s.client.try_get(&42), Err(Ok(err(Error::NotFound))));
    // over-spend fails inside the token transfer and nothing is stored
    assert!(s.client.try_create(&s.buyer, &s.seller, t, &5_000, &2_000, &m).is_err());
    assert_eq!(s.client.count(), 0);
}
