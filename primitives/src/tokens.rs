// Copyright 2021 Parallel Finance Developer.
// This file is part of Parallel Finance.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::CurrencyId;

// Native Token
pub const HKO: CurrencyId = 0;
pub const PARA: CurrencyId = 1;

// Polkadot ecosystem
pub const KSM: CurrencyId = 100;
pub const DOT: CurrencyId = 101;
pub const USDT: CurrencyId = 102;
pub const KUSD: CurrencyId = 103;
pub const AUSD: CurrencyId = 104;
pub const LC_KSM: CurrencyId = 105;
pub const LC_DOT: CurrencyId = 106;
pub const KAR: CurrencyId = 107;
pub const ACA: CurrencyId = 108;
pub const LKSM: CurrencyId = 109;
pub const LDOT: CurrencyId = 110;
pub const SDN: CurrencyId = 111;
pub const ASTR: CurrencyId = 112;
pub const MOVR: CurrencyId = 113;
pub const GLMR: CurrencyId = 114;
pub const PHA: CurrencyId = 115;
pub const KMA: CurrencyId = 117;
pub const MA: CurrencyId = 118;
pub const KINT: CurrencyId = 119;
pub const INTR: CurrencyId = 120;
pub const KBTC: CurrencyId = 121;
pub const IBTC: CurrencyId = 122;
pub const GENS: CurrencyId = 123;
pub const EQ: CurrencyId = 124;
pub const TUR: CurrencyId = 125;
pub const LIT: CurrencyId = 127;
pub const CLV: CurrencyId = 130;

// Ethereum ecosystem
pub const EUSDT: CurrencyId = 201;
pub const EUSDC: CurrencyId = 202;
pub const HBTC: CurrencyId = 203;

// Liquid Staking Derivative
pub const SKSM: CurrencyId = 1000;
pub const SDOT: CurrencyId = 1001;

// Money Market Derivative
pub const PHKO: CurrencyId = 2000;
pub const PPARA: CurrencyId = 2001;
pub const PKSM: CurrencyId = 2100;
pub const PDOT: CurrencyId = 2101;
pub const PUSDT: CurrencyId = 2102;
pub const PKUSD: CurrencyId = 2103;
pub const PAUSD: CurrencyId = 2104;
pub const PLC_KSM: CurrencyId = 2105;
pub const PLC_DOT: CurrencyId = 2106;
pub const PKAR: CurrencyId = 2107;
pub const PACA: CurrencyId = 2108;
pub const PLKSM: CurrencyId = 2109;
pub const PLDOT: CurrencyId = 2110;

pub const PEUSDT: CurrencyId = 2201;
pub const PEUSDC: CurrencyId = 2202;

pub const PSKSM: CurrencyId = 3000;
pub const PSDOT: CurrencyId = 3001;
pub const PKSM_U: CurrencyId = 3002;
pub const PDOT_U: CurrencyId = 3003;

pub const PCDOT_6_13: CurrencyId = 200062013;
pub const PCDOT_7_14: CurrencyId = 200072014;

// AMM LP Token
pub const LP_USDT_HKO: CurrencyId = 5000;
pub const LP_KSM_USDT: CurrencyId = 5001;
pub const LP_KSM_HKO: CurrencyId = 5002;
pub const LP_KSM_SKSM: CurrencyId = 5003;
pub const LP_KSM_CKSM_20_27: CurrencyId = 5004;

pub const LP_USDT_PARA: CurrencyId = 6000;
pub const LP_DOT_USDT: CurrencyId = 6001;
pub const LP_DOT_PARA: CurrencyId = 6002;
pub const LP_DOT_SDOT: CurrencyId = 6003;
pub const LP_DOT_CDOT_6_13: CurrencyId = 6004;
pub const LP_DOT_CDOT_7_14: CurrencyId = 6005;
pub const LP_PARA_CDOT_6_13: CurrencyId = 6006;
pub const LP_DOT_CDOT_8_15: CurrencyId = 6007;

pub const PLP_USDT_HKO: CurrencyId = 7000;
pub const PLP_KSM_USDT: CurrencyId = 7001;
pub const PLP_KSM_HKO: CurrencyId = 7002;
pub const PLP_KSM_SKSM: CurrencyId = 7003;
pub const PLP_KSM_CKSM_20_27: CurrencyId = 7004;

pub const PLP_USDT_PARA: CurrencyId = 8000;
pub const PLP_DOT_USDT: CurrencyId = 8001;
pub const PLP_DOT_PARA: CurrencyId = 8002;
pub const PLP_DOT_SDOT: CurrencyId = 8003;
pub const PLP_DOT_CDOT_6_13: CurrencyId = 8004;
pub const PLP_DOT_CDOT_7_14: CurrencyId = 8005;
pub const PLP_PARA_CDOT_6_13: CurrencyId = 8006;
pub const PLP_DOT_CDOT_8_15: CurrencyId = 8007;

// Crowdloan Derivative
pub const CKSM_15_22: CurrencyId = 100150022;
pub const CKSM_20_27: CurrencyId = 100200027;
pub const CKSM_21_28: CurrencyId = 100210028;
pub const CDOT_6_13: CurrencyId = 200060013;
pub const CDOT_7_14: CurrencyId = 200070014;
pub const CDOT_8_15: CurrencyId = 200080015;

// Relay Currency Auxiliary
pub const KSM_U: CurrencyId = 4294957295;
pub const DOT_U: CurrencyId = 4294957296;

// assume all vault token are liquidation free and within range here
pub fn is_vault_token(asset_id: CurrencyId) -> bool {
    asset_id > 100000000 && asset_id < 300000000
}

// we only care about liquidation fee lp tokens here
// which constructed with vault token and relay token
pub fn is_lf_lp_token(asset_id: CurrencyId) -> bool {
    (asset_id > 5003 && asset_id < 6000) || (asset_id > 6003 && asset_id < 7000)
}

pub fn is_ls_token(asset_id: CurrencyId) -> bool {
    asset_id == SKSM || asset_id == SDOT
}

pub fn is_auxiliary_token(asset_id: CurrencyId) -> bool {
    asset_id >= u32::MAX - 10000
}
// Token Registration Information
// +───────────+──────────────+────────────────────+
// | Network   | Token        | Register in block  |
// +───────────+──────────────+────────────────────+
// | Heiko     | HKO          | Native             |
// | Heiko     | KSM          | N/A                |
// | Heiko     | USDT         | N/A                |
// | Heiko     | KUSD         | N/A                |
// | Heiko     | EUSDC        | N/A                |
// | Heiko     | EUSDT        | N/A                |
// | Heiko     | KAR          | N/A                |
// | Heiko     | SKSM         | N/A                |
// | Heiko     | CKSM         | N/A                |
// | Heiko     | LKSM         | N/A                |
// | Heiko     | MOVR         | N/A                |
// | Heiko     | SDN          | N/A                |
// | Heiko     | PHA          | N/A                |
// | Heiko     | KMA          | N/A                |
// | Heiko     | KINT         | N/A                |
// | Heiko     | KBTC         | N/A                |
// | Heiko     | GENS         | N/A                |
// | Heiko     | PHKO         | N/A                |
// | Heiko     | PKSM         | N/A                |
// | Heiko     | PUSDT        | N/A                |
// | Heiko     | PKUSD        | N/A                |
// | Heiko     | PEUSDT       | N/A                |
// | Heiko     | PEUSDC       | N/A                |
// | Heiko     | PKAR         | N/A                |
// | Heiko     | PSKSM        | N/A                |
// | Heiko     | PLKSM        | N/A                |
// | Heiko     | PLCKSM       | N/A                |
// | Heiko     | PCKSM        | N/A                |
// | Parallel  | PARA         | Native             |
// | Parallel  | KSM          | N/A                |
// | Parallel  | DOT          | N/A                |
// | Parallel  | USDT         | N/A                |
// | Parallel  | AUSD         | N/A                |
// | Parallel  | EUSDC        | N/A                |
// | Parallel  | EUSDT        | N/A                |
// | Parallel  | ACA          | N/A                |
// | Parallel  | SDOT         | N/A                |
// | Parallel  | CDOT         | N/A                |
// | Parallel  | LDOT         | N/A                |
// | Parallel  | LCDOT        | N/A                |
// | Parallel  | GLMR         | N/A                |
// | Parallel  | ASTR         | N/A                |
// | Parallel  | PPARA        | Native             |
// | Parallel  | PKSM         | N/A                |
// | Parallel  | PDOT         | N/A                |
// | Parallel  | PUSDT        | N/A                |
// | Parallel  | PAUSD        | N/A                |
// | Parallel  | PEUSDC       | N/A                |
// | Parallel  | PEUSDT       | N/A                |
// | Parallel  | PACA         | N/A                |
// | Parallel  | PSDOT        | N/A                |
// | Parallel  | PLDOT        | N/A                |
// | Parallel  | PLCDOT       | N/A                |
// | Parallel  | PCDOT        | N/A                |
// +──────────+───────────────+────────────────────+
