use codec::{Decode, Encode};
use core::cmp::Ordering;
use rstd::collections::btree_map::BTreeMap;
use sp_arithmetic::traits::{SimpleArithmetic, Zero};

use crate::reward_sharing::{RewardSharing, Rewarder};

#[derive(Decode, Encode, Default, Eq, PartialEq, Clone)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct TargetData<
    VoterId: Default + Ord + Clone,
    BalanceType: Default + Copy + SimpleArithmetic + Zero,
    PeriodType: Default + SimpleArithmetic + Copy,
> {
    pub total: BalanceType,
    pub votes: BTreeMap<VoterId, BalanceType>,

    pub rewarder: Rewarder<BalanceType, PeriodType, VoterId>,
}

#[derive(PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum VoteResult<VoteType, RewardType> {
    Success(Option<RewardType>),
    Unvoted(VoteType, Option<RewardType>),
    VoteNotFound,
}

impl<
        VoterId: Default + Ord + Clone,
        BalanceType: Default + Copy + SimpleArithmetic + Zero + Clone,
        PeriodType: Default + SimpleArithmetic + Copy,
    > TargetData<VoterId, BalanceType, PeriodType>
{
    pub fn create_with_first_vote(first_voter: VoterId, balance: BalanceType) -> Self {
        let mut res = TargetData {
            total: balance.clone(),
            votes: BTreeMap::new(),
            rewarder: Rewarder::default(),
        };
        res.votes.insert(first_voter.clone(), balance);
        res.rewarder.new_voter(first_voter);
        res
    }

    pub fn vote(
        &mut self,
        account: VoterId,
        votes: BalanceType,
    ) -> VoteResult<BalanceType, BalanceType> {
        self.total += votes.clone();
        if let Some(user_balance) = self.votes.get_mut(&account) {
            let res = match self.rewarder.pop_reward(&account) {
                Some(reward) => VoteResult::Success(Some(reward * *user_balance)),
                _ => VoteResult::Success(None),
            };
            *user_balance += votes;
            self.rewarder.increment_period();
            res
        } else {
            self.votes.insert(account.clone(), votes);
            self.rewarder.new_voter(account);
            VoteResult::Success(None)
        }
    }

    pub fn unvote(
        &mut self,
        account: &VoterId,
        balance: BalanceType,
    ) -> VoteResult<BalanceType, BalanceType> {
        if let Some(user_balance) = self.votes.get_mut(&account) {
            match balance.cmp(user_balance) {
                Ordering::Greater | Ordering::Equal => self.cancel(account),
                Ordering::Less => {
                    self.total -= balance.clone();
                    let res = VoteResult::Unvoted(
                        balance,
                        self.rewarder
                            .pop_reward(account)
                            .map(|rew| rew * *user_balance),
                    );
                    *user_balance -= balance.clone();
                    self.rewarder.increment_period();
                    res
                }
            }
        } else {
            VoteResult::VoteNotFound
        }
    }

    pub fn cancel(&mut self, account: &VoterId) -> VoteResult<BalanceType, BalanceType> {
        match self.votes.remove(account) {
            Some(balance) => {
                self.rewarder.increment_period();
                self.total -= balance.clone();
                VoteResult::Unvoted(
                    balance,
                    self.rewarder.pop_reward(account).map(|rew| rew * balance),
                )
            }
            None => VoteResult::VoteNotFound,
        }
    }
}

impl<
        VoterId: Default + Ord + Clone,
        BalanceType: Default + Copy + SimpleArithmetic + Zero + Clone,
        PeriodType: Default + SimpleArithmetic + Copy,
    > RewardSharing for TargetData<VoterId, BalanceType, PeriodType>
{
    type RewardBalance = BalanceType;
    type UserId = VoterId;

    fn append_reward(&mut self, reward: Self::RewardBalance) {
        self.rewarder.append_reward(reward / self.total.clone());
    }

    fn pop_reward(&mut self, user: &Self::UserId) -> Option<Self::RewardBalance> {
        self.rewarder
            .pop_reward(user)
            .and_then(|rew| self.votes.get(&user).map(|count| *count * rew))
    }
}

#[cfg(test)]
mod tests {
    use rstd::collections::btree_map::BTreeMap;
    type Data = super::TargetData<usize, u32, u32>;
    type VR = super::VoteResult<u32, u32>;
    use super::RewardSharing;

    const ALICE: usize = 10;
    const BOB: usize = 11;
    const CARL: usize = 12;

    macro_rules! vote_assert
    {
        ($data:ident, $( ($user:ident, $balance:expr) ), * ) => {
            let mut expected = BTreeMap::new();
            $(
                expected.insert($user, $balance);
                assert_eq!($data.vote($user, $balance), VR::Success(None));
            )*
            assert_eq!(expected, $data.votes);
        };

        ($data:ident, $( ($user:ident, $balance:expr, $reward:expr) ), * ) => {
            let mut expected = BTreeMap::new();
            $(
                expected.insert($user, $balance + *$data.votes.get(&$user).unwrap_or(&0));

                assert_eq!($data.vote($user, $balance), match $reward
                    {
                        Some(reward) => VR::Success(Some(reward)),
                        None => VR::Success(None),
                    });
            )*
            assert_eq!(expected, $data.votes);
        }
    }

    macro_rules! unvote_part_assert
    {
        ($data:ident, $( ($user:ident, $balance:expr, $reward:expr) ), * ) => {
            $(
                assert_eq!($data.unvote(&$user, $balance), VR::Unvoted($balance, $reward));
            )*
        };

        ($data:ident, $( ($user:ident, $reward:expr) ), * ) => {
            $(

                let balance =$data.get(&$user).unwrap_or(0);
                assert_eq!($data.unvote(&$user, balance, VR::Unvoted(balance, $reward)));
            )*
        };
    }

    #[test]
    fn simple() {
        let data = Data::create_with_first_vote(ALICE, 100);
        assert_eq!(data.total, 100);
        assert_eq!(data.votes.len(), 1);
    }

    #[test]
    fn vote() {
        let mut data = Data::default();
        vote_assert!(data, (ALICE, 200), (BOB, 300), (CARL, 400));
    }

    #[test]
    fn reward() {
        let mut data = Data::default();
        vote_assert!(data, (ALICE, 200), (BOB, 400), (CARL, 400));
        data.append_reward(1000);

        assert_eq!(data.pop_reward(&ALICE), Some(200));

        vote_assert!(
            data,
            (ALICE, 200, None),
            (BOB, 400, Some(400)),
            (CARL, 400, Some(400))
        );
    }

    #[test]
    fn unvote() {
        let mut data = Data::default();
        vote_assert!(data, (ALICE, 400), (BOB, 400), (CARL, 400));
        data.append_reward(1200);

        unvote_part_assert!(data, (ALICE, 200, Some(400)), (BOB, 200, Some(400)));

        data.append_reward(800);

        assert_eq!(data.pop_reward(&ALICE), Some(200));
        assert_eq!(data.pop_reward(&BOB), Some(200));
        assert_eq!(data.pop_reward(&CARL), Some(400 + 400));
    }

    #[test]
    fn cancel() {
        let mut data = Data::default();
        assert_eq!(data.vote(CARL, 1000), VR::Success(None));

        assert_eq!(data.vote(ALICE, 100), VR::Success(None));
        assert_eq!(data.vote(BOB, 100), VR::Success(None));
        data.append_reward(42123);

        assert_eq!(data.vote(ALICE, 100), VR::Success(Some(3500)));
        data.append_reward(12423);

        assert_eq!(data.vote(BOB, 100), VR::Success(Some(4400)));
        data.append_reward(20423);

        assert_eq!(data.cancel(&ALICE), VR::Unvoted(200, Some(4600)));
        data.append_reward(20423);

        assert_eq!(data.cancel(&BOB), VR::Unvoted(200, Some(6200)));
        data.append_reward(20423);

        assert_eq!(data.pop_reward(&CARL), Some(95000));
        assert_eq!(data.cancel(&CARL), VR::Unvoted(1000, None));

        assert_eq!(data.pop_reward(&ALICE), None);
        assert_eq!(data.pop_reward(&BOB), None);
        assert_eq!(data.pop_reward(&CARL), None);
    }
}
