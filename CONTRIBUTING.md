# Contributing

This project has a native Rust application in development. Project code and documentation use the [MIT License](LICENSE). By submitting a contribution, you agree to license it under MIT and confirm that you have the right to do so. Preserve existing copyright and license notices. Bundled fonts, dependencies, and HustlX branding have separate terms in [third-party notices](THIRD_PARTY_NOTICES.md).

Start with the [product brief](docs/product-brief.md), [architecture proposal](docs/architecture-proposal.md), and [roadmap](docs/roadmap.md). Keep proposals and planned behavior clearly distinguished from implemented and verified behavior.

Use a small feature branch and a pull request. Target the `develop` integration branch by default. Do not push runtime changes directly to `main` or `develop`. Include the behavior change, documentation impact, and actual verification results in each PR.

Keep game operation offline, preserve historical identities, and never introduce a league, account, subscription, or network requirement into the local game workflow. Test meaningful state transitions and failure paths. Record the commands actually run; do not claim platform support from an untested build target.

Do not copy private HustlX source, credentials, match records, or unlicensed assets into this repository. Use synthetic data for examples and screenshots. Never commit local databases, exports containing personal data, `.env` files, access tokens, or signing credentials.

Build commands, formatting commands, automated checks, and current limitations are documented in [development](docs/development.md).
