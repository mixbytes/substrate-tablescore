# tablescore-pallet for Substrate

## Overview
Pallet for work with table score. 

| Target    | Score |
| --------- | ----- |
| Target 1  | 100   |
| Target 2  | 75    |
| Target 2  | 72    |
| ...       | ...   |

## Example
Example of selecting a subset of accounts by tablescore

```rust
pub trait Trait: tablescore::Trait<TargetType=AccountId<Self>> {
    ...
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		pub fn do_something(origin, table_id: <T as tablescore::Trait>::TableId) -> dispatch::DispatchResult {
			let who = ensure_signed(origin)?;

            let head = tablescore::Module::<T>::tables(table_id).get_head();
            /// Work with head

			Ok(())
        }
}
```
## Terminology
// ToDo

## Scenarios
// ToDo

## Interface
// ToDo

## Storage Items
// ToDo
