use crate::{mock::*, Trait, VoteResult};

use frame_support::assert_ok;

const HEAD_COUNT: u8 = 10;

const ALICE: <Test as system::Trait>::AccountId = 0;
const BOB: <Test as system::Trait>::AccountId = 1;
const ASSET_ID: <Test as assets::Trait>::AssetId = 0;

type TargetType = <Test as Trait>::TargetType;
const TARGET1: TargetType = 1;
const TARGET2: TargetType = 2;
const TARGET3: TargetType = 3;

#[test]
fn create()
{
    new_test_ext().execute_with(|| {
        let table = TablescoreModule::next_table_id();
        assert_ok!(TablescoreModule::create_table(
            Origin::signed(ALICE),
            ASSET_ID,
            HEAD_COUNT,
            None
        ));

        let table = TablescoreModule::tables(table);

        assert_eq!(table.name, None);
        assert_eq!(table.vote_asset, ASSET_ID);
        assert_eq!(table.wallet, ALICE);

        assert_eq!(table.scores.len(), 0);
        assert_eq!(table.targets.len(), 0);
    });
}

#[test]
fn vote()
{
    new_test_ext().execute_with(|| {
        let table = TablescoreModule::next_table_id();
        assert_ok!(TablescoreModule::create_table(
            Origin::signed(ALICE),
            ASSET_ID,
            HEAD_COUNT,
            None
        ));

        let mut table = TablescoreModule::tables(table);
        assert_eq!(table.vote(TARGET1, &ALICE, 102), VoteResult::Success(None));
        assert_eq!(table.vote(TARGET2, &BOB, 101), VoteResult::Success(None));
        assert_eq!(table.vote(TARGET3, &ALICE, 100), VoteResult::Success(None));

        let head: Vec<TargetType> = table.get_head().into_iter().map(|v| *v).collect();
        assert_eq!(head, vec![TARGET1, TARGET2, TARGET3]);

        assert_eq!(table.vote(TARGET3, &BOB, 100), VoteResult::Success(None));
        assert_eq!(table.vote(TARGET2, &BOB, 50), VoteResult::Success(None));

        let head: Vec<TargetType> = table.get_head().into_iter().map(|v| *v).collect();
        assert_eq!(head, vec![TARGET3, TARGET2, TARGET1]);

        assert_eq!(table.unvote(TARGET3, &BOB, 100), VoteResult::Unvoted(100, None));
        assert_eq!(table.unvote(TARGET2, &BOB, 50), VoteResult::Unvoted(50, None));

        let head: Vec<TargetType> = table.get_head().into_iter().map(|v| *v).collect();
        assert_eq!(head, vec![TARGET1, TARGET2, TARGET3]);

        assert_eq!(table.unvote(TARGET2, &BOB, 101), VoteResult::Unvoted(101, None));
        let head: Vec<TargetType> = table.get_head().into_iter().map(|v| *v).collect();
        assert_eq!(head, vec![TARGET1, TARGET3]);

        assert_eq!(table.unvote(TARGET2, &BOB, 1), VoteResult::VoteNotFound);
    });
}
