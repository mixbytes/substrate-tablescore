# tablescore-pallet for Substrate

## Overview
Pallet for work with table score. 

| Target    | Score |
| --------- | ----- |
| Target 1  | 100   |
| Target 2  | 75    |
| Target 2  | 72    |
| ...       | ...   |

## Description

Within the framework of one module, it is possible for us to create an arbitrary number of tables, however, having one type of goals (for example `AccountId`). Within these tables, information is stored as follows:

```rust
/// Sorted target set for look at head
pub scores: BTreeSet<Record<TargetType, BalanceType>>,
/// Targets data with voter map, total vote-balance and reward info
pub targets: BTreeMap<TargetType, TargetData<VoterId, BalanceType, PeriodType>>,
```

This storage approach always allows you to have both a sorted list of targets and `unvote`, `cancel` and `get_reward` functionality. 

In pallet public API we have methods:
```rust
/// Creating new table and emit event
pub fn create_table(origin, vote_asset: AssetId<T>, head_len: u8, name: Option<Vec<u8>>) -> dispatch::DispatchResult;

/// Vote for the target
pub fn vote(origin, table_id: T::TableId, vote: Balance<T>, target: T::TargetType) -> dispatch::DispatchResult;

/// Unvote for the target
pub fn unvote(origin, table_id: T::TableId, vote: Balance<T>, target: T::TargetType) -> dispatch::DispatchResult;

/// Cancel your vote for target
pub fn cancel(origin, table_id: T::TableId, target: T::TargetType) -> dispatch::DispatchResult;

/// Store reward for target
pub fn append_reward(origin, table_id: T::TableId, balance: Balance<T>, target: T::TargetType) -> dispatch::DispatchResult;

/// Pick up your reward for target
pub fn pop_reward(origin, table_id: T::TableId, target: T::TargetType) -> dispatch::DispatchResult;
```

Reward tokens are stored on the table creator's wallet in a reserved state (temporary solution).

## Build

```console
# Build
cargo build

# Build as wasm
cargo wbuild

# Test pallet
cargo test
```

## Example
Example of selecting a subset of accounts by tablescore

```rust
pub trait Trait: tablescore::Trait<TargetType=AccountId<Self>> {
    ...
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        pub fn work_with_head(origin, table_id: <T as tablescore::Trait>::TableId) -> dispatch::DispatchResult {
            let who = ensure_signed(origin)?;
            let head = tablescore::Module::<T>::tables(table_id).get_head();
            /// Work with head
            Ok(())
        }
}
```
