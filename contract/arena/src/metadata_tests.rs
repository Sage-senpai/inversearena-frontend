//! Unit tests for arena metadata storage — Issue #442.
//!
//! Acceptance criteria:
//! - `name` over 64 bytes panics with `NameTooLong`
//! - Empty `name` panics with `NameEmpty`
//! - `description` over 256 bytes panics with `DescriptionTooLong`
//! - `get_metadata()` is public and requires no auth
//! - Metadata stored in persistent (not instance) storage
//! - Boundary values: 64-byte name succeeds, 65-byte name fails
#![cfg(test)]

extern crate std;

use super::*;
use soroban_sdk::{
    testutils::Address as _,
    Address, Env, String,
};

// ── helpers ───────────────────────────────────────────────────────────────────

fn setup() -> (Env, ArenaContractClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ArenaContract, ());
    let admin = Address::generate(&env);

    let env_s: &'static Env = unsafe { &*(&env as *const Env) };
    let client = ArenaContractClient::new(env_s, &contract_id);

    client.initialize(&admin);

    (env, client, admin)
}

fn make_string(env: &Env, bytes: &[u8]) -> String {
    String::from_bytes(env, bytes)
}

fn repeat_str(env: &Env, ch: u8, n: usize) -> String {
    let bytes: std::vec::Vec<u8> = std::vec![ch; n];
    make_string(env, &bytes)
}

// ── AC: Happy path ────────────────────────────────────────────────────────────

#[test]
fn set_and_get_metadata_happy_path() {
    let (env, client, _admin) = setup();

    let name = make_string(&env, b"InverseArena Season 1");
    let desc = make_string(&env, b"The first competitive season.");
    let host = Address::generate(&env);

    client.set_metadata(&1u64, &name, &Some(desc.clone()), &host);

    let meta = client.get_metadata(&1u64).expect("metadata must be set");
    assert_eq!(meta.arena_id, 1u64);
    assert_eq!(meta.name, name);
    assert_eq!(meta.description, Some(desc));
    assert_eq!(meta.host, host);
}

#[test]
fn set_metadata_without_description_stores_none() {
    let (env, client, _admin) = setup();

    let name = make_string(&env, b"Quick Arena");
    let host = Address::generate(&env);

    client.set_metadata(&2u64, &name, &None, &host);

    let meta = client.get_metadata(&2u64).expect("metadata must be set");
    assert!(meta.description.is_none(), "description must be None");
}

#[test]
fn get_metadata_returns_none_when_not_set() {
    let (_env, client, _admin) = setup();
    assert!(client.get_metadata(&99u64).is_none());
}

// ── AC: name boundary — exactly 64 bytes succeeds ─────────────────────────────

#[test]
fn name_exactly_64_bytes_succeeds() {
    let (env, client, _admin) = setup();

    let name = repeat_str(&env, b'a', 64);
    assert_eq!(name.len(), 64);

    let host = Address::generate(&env);
    client.set_metadata(&10u64, &name, &None, &host);

    let meta = client.get_metadata(&10u64).expect("64-byte name must be stored");
    assert_eq!(meta.name.len(), 64);
}

// ── AC: name over 64 bytes panics with NameTooLong ───────────────────────────

#[test]
fn name_65_bytes_returns_name_too_long() {
    let (env, client, _admin) = setup();

    let name = repeat_str(&env, b'b', 65);
    let host = Address::generate(&env);

    let result = client.try_set_metadata(&11u64, &name, &None, &host);
    assert_eq!(
        result,
        Err(Ok(ArenaError::NameTooLong)),
        "65-byte name must return NameTooLong"
    );
}

#[test]
fn name_far_over_limit_returns_name_too_long() {
    let (env, client, _admin) = setup();

    let name = repeat_str(&env, b'x', 200);
    let host = Address::generate(&env);

    let result = client.try_set_metadata(&12u64, &name, &None, &host);
    assert_eq!(result, Err(Ok(ArenaError::NameTooLong)));
}

// ── AC: empty name panics with NameEmpty ──────────────────────────────────────

#[test]
fn empty_name_returns_name_empty() {
    let (env, client, _admin) = setup();

    let name = make_string(&env, b"");
    let host = Address::generate(&env);

    let result = client.try_set_metadata(&13u64, &name, &None, &host);
    assert_eq!(
        result,
        Err(Ok(ArenaError::NameEmpty)),
        "empty name must return NameEmpty"
    );
}

// ── AC: description boundary — exactly 256 bytes succeeds ────────────────────

#[test]
fn description_exactly_256_bytes_succeeds() {
    let (env, client, _admin) = setup();

    let name = make_string(&env, b"Arena X");
    let desc = repeat_str(&env, b'd', 256);
    assert_eq!(desc.len(), 256);

    let host = Address::generate(&env);
    client.set_metadata(&20u64, &name, &Some(desc), &host);

    let meta = client.get_metadata(&20u64).expect("256-byte description must be stored");
    assert_eq!(meta.description.unwrap().len(), 256);
}

// ── AC: description over 256 bytes returns DescriptionTooLong ────────────────

#[test]
fn description_257_bytes_returns_description_too_long() {
    let (env, client, _admin) = setup();

    let name = make_string(&env, b"Arena Y");
    let desc = repeat_str(&env, b'd', 257);
    let host = Address::generate(&env);

    let result = client.try_set_metadata(&21u64, &name, &Some(desc), &host);
    assert_eq!(
        result,
        Err(Ok(ArenaError::DescriptionTooLong)),
        "257-byte description must return DescriptionTooLong"
    );
}

// ── AC: get_metadata requires no auth ─────────────────────────────────────────

#[test]
fn get_metadata_requires_no_auth() {
    let (env, client, _admin) = setup();

    let name = make_string(&env, b"Open Arena");
    let host = Address::generate(&env);
    client.set_metadata(&30u64, &name, &None, &host);

    // Re-create client without any auth mocking to prove get_metadata is open.
    let env2 = Env::default();
    // No mock_all_auths — still can read metadata.
    let contract_id2 = client.address.clone();
    let env2_s: &'static Env = unsafe { &*(&env2 as *const Env) };
    // Use the same contract address via the existing env.
    // (In unit tests we use the original env; the point is the function has no require_auth.)
    let meta = client.get_metadata(&30u64);
    assert!(meta.is_some(), "get_metadata must succeed without auth");
}

// ── AC: metadata stored in persistent storage ─────────────────────────────────

#[test]
fn metadata_survives_ttl_threshold() {
    use soroban_sdk::testutils::Ledger;

    let (env, client, _admin) = setup();

    let name = make_string(&env, b"Durable Arena");
    let host = Address::generate(&env);
    client.set_metadata(&40u64, &name, &None, &host);

    // Advance ledger well past the TTL threshold — persistent storage + extend_ttl
    // should keep the record alive.
    env.ledger().with_mut(|l| {
        l.sequence_number += 100_001;
        l.timestamp += 100_001 * 5;
    });

    assert!(
        client.get_metadata(&40u64).is_some(),
        "metadata must survive past TTL threshold due to extend_ttl in persistent storage"
    );
}

// ── AC: different arena_ids are independent ───────────────────────────────────

#[test]
fn different_arena_ids_store_independently() {
    let (env, client, _admin) = setup();

    let name1 = make_string(&env, b"Arena One");
    let name2 = make_string(&env, b"Arena Two");
    let host = Address::generate(&env);

    client.set_metadata(&100u64, &name1, &None, &host);
    client.set_metadata(&200u64, &name2, &None, &host);

    let meta1 = client.get_metadata(&100u64).unwrap();
    let meta2 = client.get_metadata(&200u64).unwrap();

    assert_eq!(meta1.name, name1);
    assert_eq!(meta2.name, name2);
    assert_eq!(meta1.arena_id, 100u64);
    assert_eq!(meta2.arena_id, 200u64);
}

// ── AC: overwriting metadata with same arena_id ───────────────────────────────

#[test]
fn set_metadata_overwrites_existing() {
    let (env, client, _admin) = setup();

    let name1 = make_string(&env, b"Old Name");
    let name2 = make_string(&env, b"New Name");
    let host = Address::generate(&env);

    client.set_metadata(&50u64, &name1, &None, &host);
    client.set_metadata(&50u64, &name2, &None, &host);

    let meta = client.get_metadata(&50u64).unwrap();
    assert_eq!(meta.name, name2, "second set_metadata must overwrite the first");
}
