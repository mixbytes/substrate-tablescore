#![feature(map_first_last)]
#![cfg_attr(not(feature = "std"), no_std)]
use crate::table_data::VoteResult;
use frame_support::{decl_error, decl_event, decl_module, decl_storage, dispatch, Parameter};
use sp_arithmetic::traits::{BaseArithmetic, CheckedAdd, One, Zero};
use sp_runtime::traits::Member;
use system::{ensure_root, ensure_signed};

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

    type TableId: Default + Parameter + Member + Copy + BaseArithmetic + CheckedAdd + One;

    type TargetType: Default + Parameter + Ord + Copy;
    type PeriodType: Default + Parameter + BaseArithmetic + Copy;
}

type AssetId<T> = <T as assets::Trait>::AssetId;
type Balance<T> = <T as assets::Trait>::Balance;

type Table<T: Trait> = crate::table::Table<
    AssetId<T>,
    <T as system::Trait>::AccountId,
    <T as Trait>::TargetType,
    Balance<T>,
    <T as Trait>::PeriodType,
>;

decl_storage! {
    trait Store for Module<T: Trait> as TemplateModule
    {
        Scores get(fn tables): map hasher(blake2_256) T::TableId => Table<T>;
        TableIdSequence get(fn next_table_id): T::TableId;
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
        TableId = <T as Trait>::TableId,
    {
        TableCreated(TableId, AccountId),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait>
    {
        TableIdOverflow,
        VoteNotFound,
        NoneValue,
        StorageOverflow,
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin
    {
        type Error = Error<T>;

        fn deposit_event() = default;

        pub fn create_table(origin, vote_asset: AssetId<T>, head_len: u8, name: Option<Vec<u8>>) -> dispatch::DispatchResult
        {
            let who = ensure_signed(origin)?;
            let id = Self::get_next_table_id()?;

            Scores::<T>::insert(id, Table::<T>::new(name, head_len, vote_asset));

            Self::deposit_event(RawEvent::TableCreated(id, who));

            Ok(())
        }

        pub fn vote(origin, table_id: T::TableId, balance: Balance<T>, target: T::TargetType) -> dispatch::DispatchResult
        {
            let who = ensure_signed(origin)?;
            let table = Scores::<T>::get(table_id);
            assets::Module::<T>::reserve(&table.vote_asset, &who, balance)?;

            let mut result = VoteResult::VoteNotFound;
            Scores::<T>::mutate(&table_id, |table| { result = table.vote(target, &who, balance) });

            match result
            {
                VoteResult::Success => Ok(()),
                VoteResult::SuccessRewardOwed(reward) =>
                {
                    Self::send_reward(who, table.vote_asset, reward);
                    Ok(())
                },
                _ => Err(Error::<T>::NoneValue)?,
            }
        }

        pub fn unvote(origin, table_id: T::TableId, balance: Balance<T>, target: T::TargetType) -> dispatch::DispatchResult
        {
            let who = ensure_signed(origin)?;
            let table = Scores::<T>::get(table_id);

            let mut result = VoteResult::VoteNotFound;
            Scores::<T>::mutate(&table_id, |table| {result = table.unvote(target, &who, balance)});
            match result
            {
                VoteResult::Unvoted(unvote, reward) | VoteResult::UnvotedPart(unvote, reward) => {
                    assets::Module::<T>::unreserve(&table.vote_asset, &who, unvote);
                    if let Some(reward) = reward
                    {
                        Self::send_reward(who, table.vote_asset, reward);
                    }
                    Ok(())
                },
                VoteResult::VoteNotFound => Err(Error::<T>::VoteNotFound)?,
                _ => Err(Error::<T>::NoneValue)?,
            }
        }

        pub fn cancel(origin, table_id: T::TableId, balance: Balance<T>, target: T::TargetType) -> dispatch::DispatchResult
        {
            let who = ensure_signed(origin)?;

            let mut result = VoteResult::VoteNotFound;
            let mut asset_id = AssetId::<T>::default();

            Scores::<T>::mutate(&table_id, |table|
            {
                asset_id = table.vote_asset;
                result = table.cancel(target, &who);
            });

            match result
            {
                VoteResult::Unvoted(unvote, reward) | VoteResult::UnvotedPart(unvote, reward) =>
                {
                    assets::Module::<T>::unreserve(&asset_id, &who, unvote);
                    if let Some(reward) = reward
                    {
                        Self::send_reward(who, asset_id, reward);
                    }
                    Ok(())
                },
                VoteResult::VoteNotFound => Err(Error::<T>::VoteNotFound)?,
                _ => Err(Error::<T>::NoneValue)?,
            }
        }

        pub fn append_reward(origin, table_id: T::TableId, balance: Balance<T>, target: T::TargetType)
        {
            todo!()
        }
    }
}

impl<T: Trait> Module<T>
{
    fn get_next_table_id() -> Result<T::TableId, Error<T>>
    {
        let mut result = Err(Error::<T>::NoneValue);

        TableIdSequence::<T>::mutate(|id| match id.checked_add(&One::one())
        {
            Some(res) =>
            {
                result = Ok(*id);
                *id = res;
            }
            None =>
            {
                result = Err(Error::<T>::TableIdOverflow);
            }
        });

        result
    }

    fn send_reward(who: T::AccountId, asset_id: AssetId<T>, balance: Balance<T>)
    {
        todo!()
    }
}
