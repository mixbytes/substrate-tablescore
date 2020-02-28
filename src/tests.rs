use crate::{mock::*, Trait};

use frame_support::{assert_ok, assert_noop};

const ALICE: <Test as system::Trait>::AccountId = 0;
const ASSET_ID: <Test as assets::Trait>::AssetId = 0;

#[test]
fn test()
{
    new_test_ext().execute_with(|| {
        TablescoreModule::create_table(Origin::signed(ALICE), ASSET_ID, 10, None);
    });
}
