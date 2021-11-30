#!/usr/bin/env bash
set -e

if [ "$#" -ne 1 ]; then
	echo "Please provide the number of initial validators!"
	exit 1
fi

generate_account_id() {
	subkey inspect ${3:-} ${4:-} "$SECRET//$1/$2" | grep "Account ID" | awk '{ print $3 }'
}

generate_address() {
	subkey inspect ${3:-} ${4:-} "$SECRET//$1/$2" | grep "SS58 Address" | awk '{ print $3 }'
}

generate_address_and_account_id() {
	ACCOUNT=$(generate_account_id $1 $2 $3)
	ADDRESS=$(generate_address $1 $2 $3)


	printf "// $ADDRESS (//$1/$2)\nget_authority_keys_from_public_key(hex![\"${ACCOUNT#'0x'}\"]),"
}

V_NUM=$1

AUTHORITIES=""

for i in $(seq 1 $V_NUM); do
	AUTHORITIES+="$(generate_address_and_account_id authority $i '--scheme Sr25519')\n"
done

printf "$AUTHORITIES"