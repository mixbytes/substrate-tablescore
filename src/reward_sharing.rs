use codec::{Decode, Encode};
use rstd::collections::btree_map::BTreeMap;
use sp_arithmetic::traits::{BaseArithmetic, One, Zero};

pub trait RewardSharing {
    type RewardBalance;
    type UserId;

    fn append_reward(&mut self, reward: Self::RewardBalance);
    fn pop_reward(&mut self, user: &Self::UserId) -> Option<Self::RewardBalance>;
}

#[derive(Decode, Encode, Default, Eq, PartialEq, Clone)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Rewarder<BalanceType: BaseArithmetic, PeriodType: Ord, VoterId: Ord> {
    current_reward: BalanceType,
    rewards: BTreeMap<PeriodType, BalanceType>,
    origin: BTreeMap<VoterId, PeriodType>,
}

impl<
        BalanceType: Default + BaseArithmetic + Copy,
        PeriodType: Default + BaseArithmetic + Copy,
        VoterId: Ord,
    > Rewarder<BalanceType, PeriodType, VoterId>
{
    pub fn get_current_period(&self) -> PeriodType {
        match self.rewards.last_key_value() {
            Some((key, _value)) => *key,
            None => PeriodType::default(),
        }
    }

    fn get_next_period(&self) -> PeriodType {
        match self.rewards.last_key_value() {
            Some((key, _value)) => *key + One::one(),
            None => PeriodType::default(),
        }
    }

    pub fn new_voter(&mut self, voter: VoterId) {
        self.increment_period();
        self.origin.insert(voter, self.get_current_period());
    }

    pub fn increment_period(&mut self) {
        self.rewards
            .insert(self.get_next_period(), self.current_reward.clone());
    }
}

impl<
        BalanceType: Default + Copy + BaseArithmetic,
        PeriodType: Default + BaseArithmetic + Copy,
        VoterId: Ord,
    > RewardSharing for Rewarder<BalanceType, PeriodType, VoterId>
{
    type RewardBalance = BalanceType;
    type UserId = VoterId;

    fn append_reward(&mut self, reward: Self::RewardBalance) {
        self.current_reward += reward;
    }

    fn pop_reward(&mut self, user: &Self::UserId) -> Option<Self::RewardBalance> {
        let next_period = self.get_next_period();

        match self.origin.get_mut(user) {
            Some(start) => {
                let res = self.current_reward - *self.rewards.get(start)?;
                *start = next_period;

                if res != Zero::zero() {
                    Some(res)
                } else {
                    None
                }
            }
            None => None,
        }
    }
}

#[cfg(test)]
mod tests_reward_sharing {
    use super::RewardSharing;
    type Rewarder = super::Rewarder<u32, u8, u8>;

    const ALICE: u8 = 0;
    const BOB: u8 = 1;
    const CAROL: u8 = 2;

    #[test]
    fn simple_sharing() {
        let mut target = Rewarder::default();
        target.new_voter(ALICE);
        target.new_voter(BOB);
        target.append_reward(5);

        target.new_voter(CAROL);
        target.append_reward(1);

        assert_eq!(target.pop_reward(&ALICE), Some(6));
        assert_eq!(target.pop_reward(&BOB), Some(6));
        assert_eq!(target.pop_reward(&CAROL), Some(1));
    }
}
