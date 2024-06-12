//! Address Lookup Table program compute unit benchmark testing.

mod setup;

use {
    crate::setup::{
        close_lookup_table, create_lookup_table, deactivate_lookup_table, extend_lookup_table,
        freeze_lookup_table, TEST_CLOCK_SLOT,
    },
    mollusk_svm::Mollusk,
    mollusk_svm_bencher::MolluskComputeUnitBencher,
};

fn main() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let mut mollusk = Mollusk::new(
        &solana_address_lookup_table_program::id(),
        "solana_address_lookup_table_program",
    );

    mollusk.warp_to_slot(TEST_CLOCK_SLOT);

    MolluskComputeUnitBencher::new(mollusk)
        .bench(create_lookup_table().bench())
        .bench(freeze_lookup_table().bench())
        .bench(extend_lookup_table(0, 1).bench())
        .bench(extend_lookup_table(0, 10).bench())
        .bench(extend_lookup_table(0, 38).bench())
        .bench(extend_lookup_table(1, 2).bench())
        .bench(extend_lookup_table(1, 10).bench())
        .bench(extend_lookup_table(1, 39).bench())
        .bench(extend_lookup_table(5, 6).bench())
        .bench(extend_lookup_table(5, 15).bench())
        .bench(extend_lookup_table(5, 43).bench())
        .bench(extend_lookup_table(25, 26).bench())
        .bench(extend_lookup_table(25, 35).bench())
        .bench(extend_lookup_table(25, 63).bench())
        .bench(extend_lookup_table(50, 88).bench())
        .bench(extend_lookup_table(100, 138).bench())
        .bench(extend_lookup_table(150, 188).bench())
        .bench(extend_lookup_table(200, 238).bench())
        .bench(extend_lookup_table(255, 256).bench())
        .bench(deactivate_lookup_table().bench())
        .bench(close_lookup_table().bench())
        .must_pass(true)
        .out_dir("./benches")
        .execute();
}
