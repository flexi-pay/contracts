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
