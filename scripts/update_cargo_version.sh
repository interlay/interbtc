#!/bin/sh
TAG=$(git describe --exact-match --tags HEAD  2> /dev/null)
FOUND_TAG=$?
SED_PATTERN="s/^\\(version *= *\\).*/\\1\"$TAG\"/"

if [ $FOUND_TAG -ne 0 ]; then
    TAG=-v$(git rev-parse --short HEAD)
    SED_PATTERN="s/^\(version *= *\)\"\(.*\)\"/\1\"\2$TAG\"/"
fi

sed -i "$SED_PATTERN" $1
