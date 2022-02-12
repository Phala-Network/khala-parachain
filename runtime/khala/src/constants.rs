// Copyright (C) 2021 HashForest Technology Pte. Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! A set of constant values used in substrate runtime.

#![allow(clippy::identity_op)]

/// Money matters.
pub mod currency {
    use parachains_common::Balance;

    pub const UNIT: Balance = 1_000_000_000_000;
    pub const DOLLARS: Balance = UNIT;
    pub const CENTS: Balance = DOLLARS / 100;
    pub const MILLICENTS: Balance = CENTS / 1_000;

    pub const fn deposit(items: u32, bytes: u32) -> Balance {
        items as Balance * 15 * CENTS + (bytes as Balance) * 1 * CENTS
    }
}

/// Fee-related.
pub mod fee {
    use frame_support::weights::{
        constants::{ExtrinsicBaseWeight, WEIGHT_PER_SECOND}, WeightToFeeCoefficient, WeightToFeeCoefficients,
        WeightToFeePolynomial,
    };
    use parachains_common::Balance;
    use smallvec::smallvec;
    use sp_runtime::Perbill;

    /// The block saturation level. Fees will be updates based on this value.
    pub const TARGET_BLOCK_FULLNESS: Perbill = Perbill::from_percent(25);

    /// Handles converting a weight scalar to a fee value, based on the scale and granularity of the
    /// node's balance type.
    ///
    /// This should typically create a mapping between the following ranges:
    ///   - [0, MAXIMUM_BLOCK_WEIGHT]
    ///   - [Balance::min, Balance::max]
    ///
    /// Yet, it can be used for any other sort of change to weight-fee. Some examples being:
    ///   - Setting it to `0` will essentially disable the weight fee.
    ///   - Setting it to `1` will cause the literal `#[weight = x]` values to be charged.
    pub struct WeightToFee;
    impl WeightToFeePolynomial for WeightToFee {
        type Balance = Balance;
        fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
            // in Khala, extrinsic base weight (smallest non-zero weight) is mapped to CENTS:
            let p = super::currency::CENTS;
            let q = 100 * Balance::from(ExtrinsicBaseWeight::get());
            smallvec![WeightToFeeCoefficient {
                degree: 1,
                negative: false,
                coeff_frac: Perbill::from_rational(p % q, q),
                coeff_integer: p / q,
            }]
        }
    }

    pub fn pha_per_second() -> u128 {
        // one cent cost per tx
        let base_tx_fee = super::currency::CENTS;
        let base_weight = Balance::from(ExtrinsicBaseWeight::get());
        let tx_per_second = (WEIGHT_PER_SECOND as u128) / base_weight;
        base_tx_fee * tx_per_second
    }
}

pub mod parachains {
    pub mod karura {
        pub const ID: u32 = 2000;
        pub const KAR_KEY: [u8; 2] = [0, 128];
    }

    pub mod bifrost {
        pub const ID: u32 = 2001;
        pub const BNC_KEY: &[u8] = &[0, 1];
        pub const VSKSM_KEY: &[u8] = &[4, 4];
        pub const ZLK_KEY: &[u8] = &[2, 7];
    }
}
