pipeline {
    agent {
        kubernetes {
            yamlFile '.deploy/rust-builder-pod.yaml'
        }
    }
    environment {
        RUSTC_WRAPPER = '/usr/local/bin/sccache'
        CI = 'true'
        GITHUB_TOKEN = credentials('ns212-github-token')
    }

    options {
        timestamps()
        ansiColor('xterm')
        buildDiscarder(logRotator(artifactDaysToKeepStr: '7', artifactNumToKeepStr: '5'))
    }

    stages {
        stage('Test') {
            steps {
                container('rust') {
                    sh 'rustc --version'
                    sh 'SCCACHE_START_SERVER=1 SCCACHE_IDLE_TIMEOUT=0 /usr/local/bin/sccache'
                    sh '/usr/local/bin/sccache -s'

                    sh 'cargo fmt -- --check'
                    sh 'cargo check --workspace --release'
                    sh 'cargo test --workspace --release'

                    sh '/usr/local/bin/sccache -s'
                }
            }
        }

        stage('Build standalone') {
            steps {
                container('rust') {
                    sh 'SCCACHE_START_SERVER=1 SCCACHE_IDLE_TIMEOUT=0 /usr/local/bin/sccache'
                    sh '/usr/local/bin/sccache -s'

                    sh 'cargo build --manifest-path parachain/Cargo.toml --release --no-default-features --features aura-grandpa'

                    sh 'cp target/release/btc-parachain target/release/btc-parachain-standalone'
                    archiveArtifacts 'target/release/btc-parachain-standalone'
                    stash(name: 'build-standalone', includes: 'Dockerfile_release, target/release/btc-parachain')

                    sh '/usr/local/bin/sccache -s'
                }
            }
        }

        stage('Build parachain') {
            steps {
                container('rust') {
                    sh 'SCCACHE_START_SERVER=1 SCCACHE_IDLE_TIMEOUT=0 /usr/local/bin/sccache'
                    sh '/usr/local/bin/sccache -s'

                    sh 'cargo build --manifest-path parachain/Cargo.toml --release --no-default-features --features cumulus-polkadot'

                    archiveArtifacts 'target/release/btc-parachain'
                    stash(name: 'build-parachain', includes: 'Dockerfile_release, target/release/btc-parachain')

                    sh '/usr/local/bin/sccache -s'
                }
            }
        }

        stage('Make Image - standalone') {
            when {
                anyOf {
                    branch 'master'
                    tag '*'
                }
            }
            environment {
                PATH        = "/busybox:$PATH"
                REGISTRY    = 'registry.gitlab.com' // Configure your own registry
                REPOSITORY  = 'interlay/btc-parachain'
                IMAGE       = 'standalone'
            }
            steps {
                container(name: 'kaniko', shell: '/busybox/sh') {
                    dir('unstash') {
                        unstash('build-standalone')
                        runKaniko()
                    }
                }
            }
        }
        stage('Make Image - parachain') {
            when {
                anyOf {
                    branch 'master'
                    tag '*'
                }
            }
            environment {
                PATH        = "/busybox:$PATH"
                REGISTRY    = 'registry.gitlab.com' // Configure your own registry
                REPOSITORY  = 'interlay/btc-parachain'
                IMAGE       = 'parachain'
            }
            steps {
                container(name: 'kaniko', shell: '/busybox/sh') {
                    dir('unstash') {
                        unstash('build-parachain')
                        runKaniko()
                    }
                }
            }
        }

        stage('Create GitHub release') {
            when {
                anyOf {
                    tag '*'
                }
            }
            steps {
                sh '''
                    wget -q -O - https://github.com/git-chglog/git-chglog/releases/download/v0.10.0/git-chglog_0.10.0_linux_amd64.tar.gz | tar xzf -
                    #export PREV_TAG=$(git describe --abbrev=0 --tags `git rev-list --tags --skip=1 --max-count=1`)
                    #export TAG_NAME=$(git describe --abbrev=0 --tags `git rev-list --tags --skip=0 --max-count=1`)
                    ./git-chglog --output CHANGELOG.md $TAG_NAME
                     wget -q -O - https://github.com/cli/cli/releases/download/v1.6.2/gh_1.6.2_linux_amd64.tar.gz | tar xzf -
                    ./gh_1.6.2_linux_amd64/bin/gh auth status
                    ./gh_1.6.2_linux_amd64/bin/gh release -R $GIT_URL create $TAG_NAME --title $TAG_NAME -F CHANGELOG.md -d'
                '''
            }
        }
    }
}

def runKaniko() {
    sh '''#!/busybox/sh
    GIT_BRANCH_SLUG=$(echo $BRANCH_NAME | sed -e 's/\\//-/g')
    /kaniko/executor -f `pwd`/Dockerfile_release -c `pwd` --build-arg BINARY=btc-parachain \
        --destination=${REGISTRY}/${REPOSITORY}/${IMAGE}:${GIT_BRANCH_SLUG} \
        --destination=${REGISTRY}/${REPOSITORY}/${IMAGE}:${GIT_BRANCH_SLUG}-${GIT_COMMIT:0:6}
    '''
}
