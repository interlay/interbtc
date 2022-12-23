#!/usr/bin/env bash
set -exo pipefail

if ! [ -x "$(command -v polkadot-js-api)" ]; then
  echo 'Error: polkadot-js-api is not installed.'
  exit 1
fi

RELAYCHAIN_WSS="${RELAYCHAIN_WSS:-wss://api-testnet.interlay.io/relaychain/}"
PARACHAIN_WSS="${PARACHAIN_WSS:-wss://api-testnet.interlay.io/parachain/}"

declare -A TOKEN_IDS
TOKEN_IDS["DOT"]=0
TOKEN_IDS["INTERBTC"]=1
TOKEN_IDS["INTR"]=2
TOKEN_IDS["KSM"]=10
TOKEN_IDS["KBTC"]=11
TOKEN_IDS["KINT"]=12

WASM_FILENAME=$1
GENESIS_FILENAME=$2

if [ -z "${PARACHAIN_SEED}" ]; then
  echo "Env. PARACHAIN_SEED not set"
  exit 1
fi

function setup_parachain {
  if [ -z "${WASM_FILENAME}" ] || [ ! -f ${WASM_FILENAME:1} ]; then
    echo "WASM file not set"
    exit 1
  fi

  if [ -z "${GENESIS_FILENAME}" ] || [ ! -f ${GENESIS_FILENAME:1} ]; then
    echo "Genesis file not set"
    exit 1
  fi

  # sudoScheduleParaInitialize(id, genesis)
  polkadot-js-api --ws $RELAYCHAIN_WSS --sudo --seed "//Alice" \
      tx.parasSudoWrapper.sudoScheduleParaInitialize \
      2121 \
      "{ \"genesisHead\":\"${GENESIS_FILENAME?}\", \"validationCode\":\"${WASM_FILENAME?}\", \"parachain\": true }"

  # forceLease(para, leaser, amount, periodBegin, periodCount)
  polkadot-js-api --ws $RELAYCHAIN_WSS --sudo --seed "//Alice" \
      tx.slots.forceLease \
      2121 \
      5CiPPseXPECbkjWCa6MnjNokrgYjMqmKndv2rSnekmSK2DjL \
      0 \
      1 \
      1000
}

function setup_foreign_assets {
  # assetRegistry.registerAsset(metadata, assetId)
  polkadot-js-api --ws "${PARACHAIN_WSS}" --sudo --seed "${PARACHAIN_SEED}" \
      tx.assetRegistry.registerAsset \
      '{
        "decimals": 6,
        "name": "Tether USD",
        "symbol": "USDT", 
        "existentialDeposit": 0,
        "location": null,
        "additional": { "feePerSecond": 8153838, "coingeckoId": "tether" }
      }' \
      1
}

function setup_vault_registry {
  # TODO
}

function fund_faucet_account {
  # Fund the faucet accounts with tokens
  TOKENS=( "KSM" "KINT" )
  for T in "${TOKENS[@]}"
  do
    polkadot-js-api --ws "${PARACHAIN_WSS}" --sudo --seed "${PARACHAIN_SEED}" \
        tx.tokens.setBalance \
        5DqzGaydetDXGya818gyuHA7GAjEWRsQN6UWNKpvfgq2KyM7 \
        "{ \"Token\": ${TOKEN_IDS[$T]} }" \
        20000000000000 \
        0
  done
}

function setup_lending_markets {
  # KSM
  polkadot-js-api --ws "${PARACHAIN_WSS}" --sudo --seed "${PARACHAIN_SEED}" \
      tx.loans.addMarket \
      "{ \"Token\": ${TOKEN_IDS["KSM"]} }" \
      '{ 
        "collateralFactor": 540000,
        "liquidationThreshold": 610000,
        "reserveFactor": 200000,
        "closeFactor": 500000,
        "liquidateIncentive": "1100000000000000000",
        "liquidateIncentiveReservedFactor": 25000,
        "rateModel": {
          "Jump": {
            "baseRate": 0,
            "jumpRate": "15000000000000000",
            "fullRate": "40000000000000000",
            "jumpUtilization": 900000
          }
        },
        "state": "Active",
        "supplyCap": "30000000000000000",
        "borrowCap": "30000000000000000",
        "lendTokenId": {
          "LendToken": 0
        }
      }'

  # KBTC
  polkadot-js-api --ws "${PARACHAIN_WSS}" --sudo --seed "${PARACHAIN_SEED}" \
      tx.loans.addMarket \
      "{ \"Token\": ${TOKEN_IDS["KBTC"]} }" \
      '{ 
        "collateralFactor": 610000,
        "liquidationThreshold": 650000,
        "reserveFactor": 200000,
        "closeFactor": 500000,
        "liquidateIncentive": "1100000000000000000",
        "liquidateIncentiveReservedFactor": 25000,
        "rateModel": {
          "Jump": {
            "baseRate": 0,
            "jumpRate": "5000000000000000",
            "fullRate": "50000000000000000",
            "jumpUtilization": 900000
          }
        },
        "state": "Active",
        "supplyCap": "20000000000000",
        "borrowCap": "20000000000000",
        "lendTokenId": {
          "LendToken": 0
        }
      }'


  # KINT
  polkadot-js-api --ws "${PARACHAIN_WSS}" --sudo --seed "${PARACHAIN_SEED}" \
      tx.loans.addMarket \
      "{ \"Token\": ${TOKEN_IDS["KINT"]} }" \
      '{ 
        "collateralFactor": 610000,
        "liquidationThreshold": 650000,
        "reserveFactor": 200000,
        "closeFactor": 500000,
        "liquidateIncentive": "1100000000000000000",
        "liquidateIncentiveReservedFactor": 25000,
        "rateModel": {
          "Jump": {
            "baseRate": 0,
            "jumpRate": "5000000000000000",
            "fullRate": "50000000000000000",
            "jumpUtilization": 900000
          }
        },
        "state": "Active",
        "supplyCap": "20000000000000",
        "borrowCap": "20000000000000",
        "lendTokenId": {
          "LendToken": 0
        }
      }'
}


setup_parachain
setup_foreign_assets
setup_vault_registry
fund_faucet_account
setup_lending_markets
