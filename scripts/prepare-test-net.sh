#!/usr/bin/env bash
set -e

if [ "$#" -ne 2 ]; then
	echo "Please provide the number of initial accounts and validators!"
	exit 1
fi

generate_account_id() {
	subkey inspect ${3:-} ${4:-} "$SECRET//$1/$2" | grep "Account ID" | awk '{ print $3 }'
}

generate_address() {
	subkey inspect ${3:-} ${4:-} "$SECRET//$1/$2" | grep "SS58 Address" | awk '{ print $3 }'
}

generate_account_id_from_string() {
	ACCOUNT=$(generate_account_id $1 $2 $3)
	ADDRESS=$(generate_address $1 $2 $3)

	printf "// $ADDRESS (//$1/$2)"$'\n'"get_account_id_from_string(\"$ADDRESS\"),"
}

generate_authority_keys_from_public_key() {
	ACCOUNT=$(generate_account_id $1 $2 $3)
	ADDRESS=$(generate_address $1 $2 $3)

	printf "// $ADDRESS (//$1/$2)"$'\n'"get_authority_keys_from_public_key(hex![\"${ACCOUNT#'0x'}\"]),"
}

A_NUM=$1
ACCOUNTS=""
for i in $(seq 1 $A_NUM); do
	ACCOUNTS+="$(generate_account_id_from_string account $i '--scheme Sr25519')"$'\n'
done

V_NUM=$2
VALIDATORS=""
for i in $(seq 1 $V_NUM); do
	VALIDATORS+="$(generate_authority_keys_from_public_key authority $i '--scheme Sr25519')"$'\n'
done

SUDO="$(generate_account_id_from_string sudo 1 '--scheme Sr25519')"
ORACLE="$(generate_account_id_from_string oracle 1 '--scheme Sr25519')"

cat <<EOF
$SUDO
vec![
$VALIDATORS
],
vec![
$SUDO
$ORACLE
$ACCOUNTS
],
vec![(
$ORACLE
"Interlay".as_bytes().to_vec(),
)],
EOF