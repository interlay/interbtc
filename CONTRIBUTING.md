# Contributing

**interBTC** is an open-source software project, providing a 1:1 Bitcoin-backed asset - fully collateralized, interoperable, and censorship-resistant.

## Rules

There are a few basic ground-rules for contributors (including the maintainer(s) of the project):

- **Master** must have the latest changes and its history must never be re-written.
- **Non-master branches** must be prefixed with the *type* of ongoing work.
- **All modifications** must be made in a **pull-request** to solicit feedback from other contributors.
- **All commits** must be **signed** and have a clear commit message to better track changes.

## Workflow
We use [Github Flow](https://guides.github.com/introduction/flow/index.html), so all code changes should happen through Pull Requests.

We adopt a light-weight version of the [conventional commit message standard](https://www.conventionalcommits.org/en/v1.0.0/) for commit messages. This allows us to automatically create a CHANGELOG file.

## Issues
We use GitHub issues to track feature requests and bugs.

### Bug Reports
Please provide as much detail as possible, see the GitHub templates if you are unsure.

## Coding Style
Each change must pass the [rustfmt](https://github.com/rust-lang/rustfmt) style guidelines.

## Releases
Declaring formal releases remains the prerogative of the project maintainer(s).

We adopt [semantic versioning](https://semver.org/) for each new tag, all crates are updated together.

## License
By contributing, you agree that your contributions will be licensed under its Apache License 2.0.
