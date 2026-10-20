# HustlX OSS agent instructions

Read `CONTRIBUTING.md` and `docs/development.md` before changing this repository.

- CI/CD is disabled to save cost as of 2026-10-20. Run checks locally with `bash scripts/check.sh` on macOS or `./scripts/check.ps1` in Windows PowerShell. Include actual local results and host OS in PRs.
- Do not add or restore hosted workflows, dispatch remote builds, enable GitHub Actions, or automate publishing unless the user explicitly asks to resume CI/CD. Existing release helper code and older plans do not authorize hosted runs.
- Use a feature branch and a PR targeting `develop`; never push directly to `main` or `develop`. Local verification remains required with hosted checks disabled.
- Version preparation and native packaging remain local. The checker only previews packaging and never publishes. Follow `docs/releases.md` for manual release evidence.
- Preserve offline game operation, historical identities and operator data. Use disposable libraries and synthetic data for testing.
- Never add AI authorship credit, secrets, credentials, local databases or personal exports to commits.

In the umbrella workspace, shared policy lives in `.agents/AGENTS.md` and `docs/ai-workflow.md` at the workspace root. This repository's local-only policy takes precedence over generic hosted CI guidance. Edit this canonical file; root `AGENTS.md` is an AgentSync-managed symlink.
