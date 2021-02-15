pipeline {
    agent {
        kubernetes {
            yamlFile '.deploy/rust-builder-pod.yaml'
        }
    }
    environment {
        RUSTC_WRAPPER = '/usr/local/bin/sccache'
    }

    options {
        gitLabConnection 'Gitlab-Interlay'
        gitlabBuilds(builds: ['test', 'build-standalone', 'build-parachain'])
    }

    stages {
        stage('Test') {
            steps {
                container('rust') {
                    updateGitlabCommitStatus name: 'test', state: 'running'

                    sh 'rustc --version'
                    sh 'SCCACHE_START_SERVER=1 SCCACHE_IDLE_TIMEOUT=0 /usr/local/bin/sccache'
                    sh '/usr/local/bin/sccache -s'

                    sh 'cargo fmt -- --check'
                    sh 'cargo check --workspace --release'
                    sh 'cargo test --workspace --release'

                    sh '/usr/local/bin/sccache -s'
                }
            }
            post {
                success {
                    updateGitlabCommitStatus name: 'test', state: 'success'
                }
                failure {
                    updateGitlabCommitStatus name: 'test', state: 'failed'
                }
                unstable {
                    updateGitlabCommitStatus name: 'test', state: 'failed'
                }
                aborted {
                    updateGitlabCommitStatus name: 'test', state: 'canceled'
                }
            }
        }

        stage('Build standalone') {
            steps {
                container('rust') {
                    updateGitlabCommitStatus name: 'build-standalone', state: 'running'

                    sh 'SCCACHE_START_SERVER=1 SCCACHE_IDLE_TIMEOUT=0 /usr/local/bin/sccache'
                    sh '/usr/local/bin/sccache -s'
                    sh 'env'

                    sh 'cargo build --manifest-path parachain/Cargo.toml --release --no-default-features --features aura-grandpa'

                    sh 'cp target/release/btc-parachain target/release/btc-parachain-standalone'
                    archiveArtifacts 'target/release/btc-parachain-standalone'
                    stash(name: "build-standalone", includes: 'Dockerfile_release, target/release/btc-parachain')

                    sh '/usr/local/bin/sccache -s'
                }
            }
            post {
                success {
                    updateGitlabCommitStatus name: 'build-standalone', state: 'success'
                }
                failure {
                    updateGitlabCommitStatus name: 'build-standalone', state: 'failed'
                }
                unstable {
                    updateGitlabCommitStatus name: 'build-standalone', state: 'failed'
                }
                aborted {
                    updateGitlabCommitStatus name: 'build-standalone', state: 'canceled'
                }
            }
        }

        stage('Build parachain') {
            steps {
                container('rust') {
                    updateGitlabCommitStatus name: 'build-parachain', state: 'running'

                    sh 'SCCACHE_START_SERVER=1 SCCACHE_IDLE_TIMEOUT=0 /usr/local/bin/sccache'
                    sh '/usr/local/bin/sccache -s'

                    sh 'cargo build --manifest-path parachain/Cargo.toml --release --no-default-features --features cumulus-polkadot'

                    archiveArtifacts 'target/release/btc-parachain'
                    stash(name: "build-parachain", includes: 'Dockerfile_release, target/release/btc-parachain')

                    sh '/usr/local/bin/sccache -s'
                }
            }
            post {
                success {
                    updateGitlabCommitStatus name: 'build-parachain', state: 'success'
                }
                failure {
                    updateGitlabCommitStatus name: 'build-parachain', state: 'failed'
                }
                unstable {
                    updateGitlabCommitStatus name: 'build-parachain', state: 'failed'
                }
                aborted {
                    updateGitlabCommitStatus name: 'build-parachain', state: 'canceled'
                }
            }
        }

        stage('Make Image - standalone') {
            when {
                anyOf {
                    branch 'master'
                    branch 'dev'
                    branch 'jenkins'
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
                        unstash("build-standalone")
                        runKaniko()
                    }
                }
            }
        }
        stage('Make Image - parachain') {
            when {
                anyOf {
                    branch 'master'
                    branch 'dev'
                    branch 'jenkins'
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
                        unstash("build-parachain")
                        runKaniko()
                    }
                }
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
