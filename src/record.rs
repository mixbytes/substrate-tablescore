use codec::{Decode, Encode};
use core::cmp::{Ord, Ordering, PartialOrd};

use sp_arithmetic::traits::BaseArithmetic;

#[derive(Encode, Default, Decode, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Record<TargetType: Default, BalanceType: BaseArithmetic + Default>
{
    target: TargetType,
    pub balance: BalanceType,
}

impl<TargetType: Default, BalanceType: BaseArithmetic + Default> Record<TargetType, BalanceType>
{
    pub fn new(target: TargetType, balance: BalanceType) -> Self
    {
        Record { target, balance }
    }

    pub fn get_target(&self) -> &TargetType
    {
        &self.target
    }
}

impl<TargetType: Default + Ord, BalanceType: BaseArithmetic + Default> Ord
    for Record<TargetType, BalanceType>
{
    fn cmp(&self, other: &Self) -> Ordering
    {
        match self.balance.cmp(&other.balance)
        {
            Ordering::Equal => self.target.cmp(&other.target),
            Ordering::Greater => Ordering::Less,
            Ordering::Less => Ordering::Greater,
        }
    }
}

impl<TargetType: Default + Ord, BalanceType: BaseArithmetic + Default> PartialOrd
    for Record<TargetType, BalanceType>
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering>
    {
        Some(self.cmp(&other))
    }
}
