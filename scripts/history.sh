#!/bin/bash
#set -x

FROM_VER=$1
TO_VER=$2

DEPENDENCIES=(substrate polkadot cumulus)

DEP_PATTERN='(https:\/\/.*)\?.*#([0-9a-f]*)'

for DEP in "${DEPENDENCIES[@]}"
do
    FROM_URL=$(git show $FROM_VER:./Cargo.lock | grep $DEP? | head -1 | grep -o '".*"')
    [[ $FROM_URL =~ $DEP_PATTERN ]]
    FROM_COMMIT="${BASH_REMATCH[2]}"

    TO_URL=$(git show $TO_VER:./Cargo.lock | grep $DEP? | head -1 | grep -o '".*"')
    [[ $TO_URL =~ $DEP_PATTERN ]]
    UNTIL_COMMIT="${BASH_REMATCH[2]}"

    echo "${BASH_REMATCH[1]}/compare/$FROM_COMMIT...$UNTIL_COMMIT"
done
