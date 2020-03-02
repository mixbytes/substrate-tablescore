use rstd::collections::{btree_map::BTreeMap, btree_set::BTreeSet};

use rstd::prelude::Vec;

use crate::record::*;
use crate::reward_sharing::RewardSharing;
use crate::table_data::*;
use codec::{Decode, Encode};
use sp_arithmetic::traits::{BaseArithmetic, Zero};

pub type RawString = Vec<u8>;

#[derive(Decode, Encode, Default, Eq, PartialEq, Clone)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Table<
    AssetId: Default + Encode + Decode,
    VoterId: Default + Ord + Encode + Decode + Clone,
    TargetType: Default + Ord + Encode + Decode,
    BalanceType: Default + Copy + BaseArithmetic + Zero + Encode + Decode,
    PeriodType: Default + BaseArithmetic + Copy + Encode + Decode,
    WalletType: Default + Encode + Decode,
> {
    /// Optional name for table
    pub name: Option<RawString>,

    /// Count for method `get_head`
    head_count: u8,

    /// Asset for vote and reward
    pub vote_asset: AssetId,

    /// Sorted target set for look at head
    pub scores: BTreeSet<Record<TargetType, BalanceType>>,

    /// Targets data with voter map, total vote-balance and reward info
    pub targets: BTreeMap<TargetType, TargetData<VoterId, BalanceType, PeriodType>>,

    /// Wallet for lock reward tokens before send
    pub wallet: WalletType,
}

impl<
        AssetId: Default + Encode + Decode,
        VoterId: Default + Ord + Encode + Decode + Clone,
        TargetType: Default + Ord + Copy + Encode + Decode,
        BalanceType: Default + Copy + BaseArithmetic + Clone + Encode + Decode,
        PeriodType: Default + BaseArithmetic + Copy + Encode + Decode,
        WalletType: Default + Encode + Decode,
    > Table<AssetId, VoterId, TargetType, BalanceType, PeriodType, WalletType>
{
    pub fn new(
        name: Option<RawString>,
        head_count: u8,
        vote_asset: AssetId,
        wallet: WalletType,
    ) -> Self
    {
        Table {
            name,
            head_count,
            vote_asset,
            wallet,
            scores: BTreeSet::default(),
            targets: BTreeMap::default(),
        }
    }

    fn update_record(
        &mut self,
        target: TargetType,
        old_balance: BalanceType,
        new_balance: BalanceType,
    )
    {
        let mut rec = Record::new(target, old_balance);
        self.scores.remove(&rec);
        if new_balance != Zero::zero()
        {
            rec.balance = new_balance;
            self.scores.insert(rec);
        }
    }

    fn process<F>(
        &mut self,
        target: TargetType,
        account: &VoterId,
        balance: BalanceType,
        is_insert: bool,
        callback: F,
    ) -> VoteResult<BalanceType>
    where
        F: FnOnce(&mut TargetData<VoterId, BalanceType, PeriodType>) -> VoteResult<BalanceType>,
    {
        let (result, old_balance, new_balance) = match self.targets.get_mut(&target)
        {
            Some(data) =>
            {
                let old_balance = data.total.clone();
                let res = callback(data);

                (res, old_balance, data.total.clone())
            }
            None =>
            {
                if is_insert && balance != Zero::zero()
                {
                    self.targets.insert(
                        target,
                        TargetData::create_with_first_vote(account.clone(), balance.clone()),
                    );
                    (VoteResult::Success(None), Zero::zero(), balance)
                }
                else
                {
                    (VoteResult::VoteNotFound, Zero::zero(), balance)
                }
            }
        };

        match &result
        {
            VoteResult::VoteNotFound =>
            {}
            VoteResult::Unvoted(_unvoted, _reward) =>
            {
                if new_balance == Zero::zero()
                {
                    self.targets.remove(&target);
                }

                self.update_record(target, old_balance, new_balance);
            }
            _ => self.update_record(target, old_balance, new_balance),
        }

        result
    }

    pub fn vote(
        &mut self,
        target: TargetType,
        voter: &VoterId,
        balance: BalanceType,
    ) -> VoteResult<BalanceType>
    {
        self.process(target, voter, balance.clone(), true, |td| {
            td.vote(voter.clone(), balance)
        })
    }

    pub fn unvote(
        &mut self,
        target: TargetType,
        voter: &VoterId,
        balance: BalanceType,
    ) -> VoteResult<BalanceType>
    {
        self.process(target, voter, balance.clone(), false, |td| {
            td.unvote(voter, balance)
        })
    }

    pub fn cancel(&mut self, target: TargetType, account: &VoterId) -> VoteResult<BalanceType>
    {
        self.process(target, account, Zero::zero(), false, |td| td.cancel(account))
    }

    pub fn get_head(&self) -> Vec<&TargetType>
    {
        self.scores
            .iter()
            .take(self.head_count as usize)
            .map(|record| record.get_target())
            .collect()
    }

    pub fn pop_reward(&mut self, user: &VoterId, target: TargetType) -> Option<BalanceType>
    {
        self.targets
            .get_mut(&target)
            .and_then(|data| data.pop_reward(user))
    }

    pub fn append_reward(&mut self, target: TargetType, reward: BalanceType) -> Result<(), ()>
    {
        if let Some(data) = self.targets.get_mut(&target)
        {
            data.append_reward(reward);
            Ok(())
        }
        else
        {
            Err(())
        }
    }
}

#[cfg(test)]
mod tests
{
    type Table = super::Table<u8, u8, u8, u32, u32, u8>;
    type VR = super::VoteResult<u32>;

    const ALICE: u8 = 10;
    const BOB: u8 = 11;
    const CARL: u8 = 12;
    const CAROL: u8 = 13;

    const WALLET: u8 = 0;

    fn compare_head(table: &Table, expected: Vec<u8>)
    {
        assert_eq!(
            expected,
            table.get_head().into_iter().cloned().collect::<Vec<u8>>(),
        );
    }

    #[test]
    fn create()
    {
        let table = Table::new(None, 2, 0, WALLET);
        assert_eq!(table.get_head().len(), 0);
    }

    #[test]
    fn simple_vote()
    {
        let mut table = Table::new(None, 2, 0, WALLET);
        assert_eq!(table.vote(0, &ALICE, 10), VR::Success(None));
        assert_eq!(table.vote(1, &BOB, 11), VR::Success(None));
        assert_eq!(table.vote(2, &CARL, 12), VR::Success(None));

        compare_head(&table, vec![2, 1]);
    }

    #[test]
    fn supplement_vote()
    {
        let mut table = Table::new(None, 2, 0, WALLET);

        assert_eq!(table.vote(0, &ALICE, 10), VR::Success(None));
        assert_eq!(table.vote(1, &BOB, 11), VR::Success(None));
        assert_eq!(table.vote(2, &CARL, 12), VR::Success(None));
        assert_eq!(table.vote(3, &CAROL, 13), VR::Success(None));

        assert_eq!(table.vote(0, &ALICE, 10), VR::Success(None));
        assert_eq!(table.vote(1, &BOB, 4), VR::Success(None));
        assert_eq!(table.vote(2, &CARL, 4), VR::Success(None));
        assert_eq!(table.vote(3, &CAROL, 6), VR::Success(None));

        compare_head(&table, vec![0, 3])
    }

    #[test]
    fn unvote()
    {
        let mut table = Table::new(None, 3, 0, WALLET);

        assert_eq!(table.vote(1, &ALICE, 5), VR::Success(None));
        assert_eq!(table.vote(2, &BOB, 10), VR::Success(None));
        assert_eq!(table.vote(3, &CAROL, 20), VR::Success(None));

        compare_head(&table, vec![3, 2, 1]);

        assert_eq!(table.unvote(3, &CAROL, 11), VR::Unvoted(11, None));
        compare_head(&table, vec![2, 3, 1]);

        assert_eq!(table.unvote(2, &BOB, 6), VR::Unvoted(6, None));
        compare_head(&table, vec![3, 1, 2]);
    }

    #[test]
    fn multivote()
    {
        let mut table = Table::new(None, 2, 0, WALLET);

        assert_eq!(table.vote(0, &ALICE, 10), VR::Success(None));
        assert_eq!(table.vote(1, &BOB, 11), VR::Success(None));
        assert_eq!(table.vote(2, &CARL, 12), VR::Success(None));
        assert_eq!(table.vote(3, &CAROL, 13), VR::Success(None));

        compare_head(&table, vec![3, 2]);

        assert_eq!(table.vote(1, &ALICE, 10), VR::Success(None));
        compare_head(&table, vec![1, 3]);

        assert_eq!(table.vote(2, &ALICE, 8), VR::Success(None));
        compare_head(&table, vec![1, 2]);
    }

    #[test]
    fn cancel_vote()
    {
        let mut table = Table::new(None, 2, 0, WALLET);

        assert_eq!(table.vote(0, &ALICE, 10), VR::Success(None));
        assert_eq!(table.vote(1, &BOB, 11), VR::Success(None));
        assert_eq!(table.vote(2, &CARL, 12), VR::Success(None));
        assert_eq!(table.vote(3, &CAROL, 13), VR::Success(None));

        compare_head(&table, vec![3, 2]);

        assert_eq!(table.cancel(3, &CAROL), VR::Unvoted(13, None));
        assert_eq!(table.cancel(3, &CAROL), VR::VoteNotFound);

        compare_head(&table, vec![2, 1]);
    }

    // ToDo add reward sharing tests
}
