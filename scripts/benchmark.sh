#!/bin/bash
#set -euxo pipefail

while getopts ":r:p:" opt; do
  case $opt in
    r)
      runtime="$OPTARG"
    ;;
    p)
      pallet="$OPTARG"
    ;;
    \?) echo "Invalid option -$OPTARG" >&2
    exit 1
    ;;
  esac

  case $OPTARG in
    -*) echo "Option $opt needs a valid argument"
    exit 1
    ;;
  esac
done

if [ -z "${pallet}" ]; then
  pallet="*"
fi

cargo run \
  --bin interbtc-parachain \
  --features runtime-benchmarks \
  --release \
  -- \
  benchmark pallet \
  --pallet "${pallet}" \
  --extrinsic '*' \
  --chain "${runtime}" \
  --execution=wasm \
  --wasm-execution=compiled \
  --steps 50 \
  --repeat 10 \
  --output "parachain/runtime/${runtime}/src/weights/" \
  --template .deploy/runtime-weight-template.hbs
