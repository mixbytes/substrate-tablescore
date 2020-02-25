#![feature(map_first_last)]
#![cfg_attr(not(feature = "std"), no_std)]
use frame_support::{decl_error, decl_event, decl_module, decl_storage, dispatch, Parameter};
use sp_arithmetic::traits::{BaseArithmetic, Zero};
use sp_runtime::traits::Member;
use system::ensure_signed;

mod record;
mod reward_sharing;
mod table;
mod table_data;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub trait Trait: system::Trait + assets::Trait
{
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    type TableId: Default + Parameter + Member + Copy + BaseArithmetic;

    type TargetType: Default + Parameter + Ord;
    type PeriodType: Default + Parameter + BaseArithmetic + Copy;
}

type Table<T: Trait> = crate::table::Table<
    <T as assets::Trait>::AssetId,
    <T as system::Trait>::AccountId,
    <T as Trait>::TargetType,
    <T as assets::Trait>::Balance,
    <T as Trait>::PeriodType,
>;

decl_storage! {
    trait Store for Module<T: Trait> as TemplateModule
    {
        Scores get(fn tables): map hasher(blake2_256) T::TableId => Table<T>;
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
    {
        SomethingStored(u32, AccountId),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait>
    {
        NoneValue,
        StorageOverflow,
    }
}

decl_module! {
    /// The module declaration.
    pub struct Module<T: Trait> for enum Call where origin: T::Origin
    {
        type Error = Error<T>;

        fn deposit_event() = default;

    }
}
