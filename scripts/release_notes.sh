#!/bin/bash
git-chglog --output CHANGELOG.md $GITHUB_REF_NAME
PREV_TAG=$(git describe --abbrev=0 --tags $(git rev-list --tags --skip=1 --max-count=1))
(
        echo $'\n## Dependency changes'
        bash scripts/history.sh $PREV_TAG $GITHUB_REF_NAME
        echo $'\n## Runtimes'
        echo $'\n### Interlay'
        python3 scripts/srtool_info.py artifacts/interlay-srtool-json/interlay_srtool_output.json
        echo $'\n### Kintsugi'
        python3 scripts/srtool_info.py artifacts/kintsugi-srtool-json/kintsugi_srtool_output.json
) >>CHANGELOG.md
