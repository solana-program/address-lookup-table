//! Address Lookup Table program compute unit benchmark testing.

mod setup;

use {
    crate::setup::{
        close_lookup_table, create_lookup_table, deactivate_lookup_table, extend_lookup_table,
        freeze_lookup_table,
    },
    mollusk::Mollusk,
    mollusk_bencher::MolluskComputeUnitBencher,
    solana_sdk::clock::Clock,
};

// Taken from `https://github.com/anza-xyz/agave/blob/3e077b7350f52451dcf763f0ba05de64f34cac01/programs/address-lookup-table/src/processor.rs#L22`.
pub const DEFAULT_COMPUTE_UNITS: u64 = 750;

pub const TEST_CLOCK_SLOT: u64 = 100_000;

fn main() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");
    let mut mollusk = Mollusk::new(
        &solana_address_lookup_table_program::id(),
        "solana_address_lookup_table_program",
    );

    mollusk.sysvar_cache.set_clock(Clock {
        slot: TEST_CLOCK_SLOT,
        ..Default::default()
    });

    MolluskComputeUnitBencher::new(mollusk)
        .benchmark(DEFAULT_COMPUTE_UNITS)
        .bench(create_lookup_table())
        .bench(freeze_lookup_table())
        .bench(extend_lookup_table(0, 1))
        .bench(extend_lookup_table(0, 10))
        .bench(extend_lookup_table(0, 38))
        .bench(extend_lookup_table(1, 2))
        .bench(extend_lookup_table(1, 10))
        .bench(extend_lookup_table(1, 39))
        .bench(extend_lookup_table(5, 6))
        .bench(extend_lookup_table(5, 15))
        .bench(extend_lookup_table(5, 43))
        .bench(extend_lookup_table(25, 26))
        .bench(extend_lookup_table(25, 35))
        .bench(extend_lookup_table(25, 63))
        .bench(extend_lookup_table(50, 88))
        .bench(extend_lookup_table(100, 138))
        .bench(extend_lookup_table(150, 188))
        .bench(extend_lookup_table(200, 238))
        .bench(extend_lookup_table(255, 256))
        .bench(deactivate_lookup_table())
        .bench(close_lookup_table())
        .iterations(100)
        .must_pass(true)
        .out_dir("../target/benches")
        .execute();
}
