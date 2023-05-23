//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-05-04 (Y/M/D)
//! HOSTNAME: `sander-dell`, CPU: `11th Gen Intel(R) Core(TM) i7-11800H @ 2.30GHz`
//!
//! SHORT-NAME: `block`, LONG-NAME: `BlockExecution`, RUNTIME: `interBTC`
//! WARMUPS: `100`, REPEAT: `10`
//! WEIGHT-PATH: `parachain/runtime/testnet-kintsugi/src/weights/`
//! WEIGHT-METRIC: `Average`, WEIGHT-MUL: `1.0`, WEIGHT-ADD: `0`

// Executed Command:
//   target/release/interbtc-parachain
//   benchmark
//   overhead
//   --chain
//   kintsugi-testnet-bench
//   --execution=wasm
//   --wasm-execution=compiled
//   --warmup
//   100
//   --repeat
//   10
//   --weight-path
//   parachain/runtime/testnet-kintsugi/src/weights/

use sp_core::parameter_types;
use sp_weights::{constants::WEIGHT_REF_TIME_PER_NANOS, Weight};

parameter_types! {
    /// Time to execute an empty block.
    /// Calculated by multiplying the *Average* with `1.0` and adding `0`.
    ///
    /// Stats nanoseconds:
    ///   Min, Max: 8_566_742, 10_095_482
    ///   Average:  8_970_337
    ///   Median:   8_721_366
    ///   Std-Dev:  480372.73
    ///
    /// Percentiles nanoseconds:
    ///   99th: 10_095_482
    ///   95th: 10_095_482
    ///   75th: 9_169_046
    pub const BlockExecutionWeight: Weight =
        Weight::from_parts(WEIGHT_REF_TIME_PER_NANOS.saturating_mul(8_970_337), 0u64);
}

#[cfg(test)]
mod test_weights {
    use sp_weights::constants;

    /// Checks that the weight exists and is sane.
    // NOTE: If this test fails but you are sure that the generated values are fine,
    // you can delete it.
    #[test]
    fn sane() {
        let w = super::BlockExecutionWeight::get();

        // At least 100 µs.
        assert!(
            w.ref_time() >= 100u64 * constants::WEIGHT_REF_TIME_PER_MICROS,
            "Weight should be at least 100 µs."
        );
        // At most 50 ms.
        assert!(
            w.ref_time() <= 50u64 * constants::WEIGHT_REF_TIME_PER_MILLIS,
            "Weight should be at most 50 ms."
        );
    }
}
