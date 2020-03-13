#![feature(map_first_last)]
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{decl_error, decl_event, decl_module, decl_storage, dispatch, Parameter};
use sp_arithmetic::traits::{BaseArithmetic, CheckedAdd, One};
use sp_runtime::traits::Member;
use system::ensure_signed;

use crate::table_data::VoteResult;
use rstd::prelude::Vec;

mod record;
mod reward_sharing;
mod table;
mod table_data;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub trait Trait: system::Trait + assets::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    type TableId: Default + Parameter + Member + Copy + BaseArithmetic + CheckedAdd + One;

    type PeriodType: Default + Parameter + BaseArithmetic + Copy;

    /// Target for vote
    type TargetType: Default + Parameter + Ord + Clone;
}

type AssetId<T> = <T as assets::Trait>::AssetId;
type Balance<T> = <T as assets::Trait>::Balance;
type AccountId<T> = <T as system::Trait>::AccountId;

type Table<T> = crate::table::Table<
    AssetId<T>,
    AccountId<T>,
    <T as Trait>::TargetType,
    Balance<T>,
    <T as Trait>::PeriodType,
    <T as system::Trait>::AccountId,
>;

decl_storage! {
    trait Store for Module<T: Trait> as TemplateModule {
        /// Tables by id
        pub Scores get(fn tables): map hasher(blake2_256) T::TableId => Table<T>;

        /// Sequence for table id
        TableIdSequence get(fn next_table_id): T::TableId;
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
        TableId = <T as Trait>::TableId,
        //TargetType = <T as Trait>::TargetType,
    {
        TableCreated(TableId, AccountId),
        //ChangeVote(TableId, TargetType),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
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

        /// Creating new table and emit event
        pub fn create_table(origin, vote_asset: AssetId<T>, head_len: u8, name: Option<Vec<u8>>) -> dispatch::DispatchResult {
            let who = ensure_signed(origin)?;
            let id = Self::create(who.clone(), vote_asset, head_len, name)?;
            Self::deposit_event(Event::<T>::TableCreated(id, who));

            Ok(())
        }

        /// Vote for the target
        pub fn vote(origin, table_id: T::TableId, vote: Balance<T>, target: T::TargetType) -> dispatch::DispatchResult {
            let who = ensure_signed(origin)?;
            let table = Scores::<T>::get(table_id);
            assets::Module::<T>::reserve(&table.vote_asset, &who, vote)?;

            //Self::deposit_event(Event::<T>::ChangeVote(table_id, target.clone()));

            match Scores::<T>::mutate(&table_id, |table| table.vote(target, &who, vote)) {
                VoteResult::Success(Some(reward)) => Self::send_reward(&table.vote_asset, &table.wallet, &who, reward),
                VoteResult::Success(None) => Ok(()),
                _ => Err(Error::<T>::NoneValue)?,
            }
        }

        /// Unvote for the target
        pub fn unvote(origin, table_id: T::TableId, vote: Balance<T>, target: T::TargetType) -> dispatch::DispatchResult {
            let who = ensure_signed(origin)?;
            let table = Scores::<T>::get(table_id);

            //Self::deposit_event(Event::<T>::ChangeVote(table_id, target.clone()));

            match Scores::<T>::mutate(&table_id, |table| table.unvote(target, &who, vote)) {
                VoteResult::Unvoted(unvote, reward) => {
                    assets::Module::<T>::unreserve(&table.vote_asset, &who, unvote);
                    if let Some(reward) = reward {
                        Self::send_reward(&table.vote_asset, &table.wallet, &who, reward)?;
                    }
                    Ok(())
                },
                VoteResult::VoteNotFound => Err(Error::<T>::VoteNotFound)?,
                _ => Err(Error::<T>::NoneValue)?,
            }
        }

        /// Cancel your vote for target
        pub fn cancel(origin, table_id: T::TableId, target: T::TargetType) -> dispatch::DispatchResult
        {
            let who = ensure_signed(origin)?;

            let table = Scores::<T>::get(table_id);
            match Scores::<T>::mutate(&table_id, |table| table.cancel(target, &who)) {
                VoteResult::Unvoted(unvote, reward) => {
                    assets::Module::<T>::unreserve(&table.vote_asset, &who, unvote);
                    if let Some(reward) = reward {
                        Self::send_reward(&table.vote_asset, &table.wallet, &who, reward)?;
                    }
                    Ok(())
                },
                VoteResult::VoteNotFound => Err(Error::<T>::VoteNotFound)?,
                _ => Err(Error::<T>::NoneValue)?,
            }
        }

        /// Store reward for target
        pub fn append_reward(origin, table_id: T::TableId, balance: Balance<T>, target: T::TargetType) -> dispatch::DispatchResult {
            let who = ensure_signed(origin)?;
            let table = Scores::<T>::get(table_id);

            assets::Module::<T>::make_transfer(&table.vote_asset, &who, &table.wallet, balance)?;
            assets::Module::<T>::reserve(&table.vote_asset, &table.wallet, balance)?;

            Scores::<T>::mutate(&table_id, |table|  table.append_reward(target, balance) ).map_err(|_| Error::<T>::NoneValue)?;

            Ok(())
        }

        /// Pick up your reward for target
        pub fn pop_reward(origin, table_id: T::TableId, target: T::TargetType) -> dispatch::DispatchResult {
            let who = ensure_signed(origin)?;
            let table = Scores::<T>::get(table_id);

            if let Some(reward) = Scores::<T>::mutate(&table_id, |table| table.pop_reward(&who, target)) {
                Self::send_reward(&table.vote_asset, &table.wallet, &who, reward)?;
            }
            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
    pub fn create(
        who: AccountId<T>,
        vote_asset: AssetId<T>,
        head_len: u8,
        name: Option<Vec<u8>>,
    ) -> Result<T::TableId, Error<T>> {
        let id = Self::get_next_table_id()?;
        Scores::<T>::insert(id, Table::<T>::new(name, head_len, vote_asset, who)); // ToDo create normal wallet
        Ok(id)
    }

    fn get_next_table_id() -> Result<T::TableId, Error<T>> {
        TableIdSequence::<T>::mutate(|id| match id.checked_add(&One::one()) {
            Some(res) => {
                let result = *id;
                *id = res;
                Ok(result)
            }
            None => Err(Error::<T>::TableIdOverflow),
        })
    }

    fn send_reward(
        asset_id: &AssetId<T>,
        wallet: &T::AccountId,
        who: &T::AccountId,
        balance: Balance<T>,
    ) -> dispatch::DispatchResult {
        assets::Module::<T>::make_transfer(
            asset_id,
            wallet,
            who,
            assets::Module::<T>::unreserve(asset_id, wallet, balance),
        )
    }
}
