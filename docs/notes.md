# Engineering decisions

## 2026-10-20: Local checks only

CI/CD is paused for `hustlx-oss` to save hosted runner cost. Repository GitHub Actions is disabled, active and queued runs were cancelled, and the Rust and desktop release workflows are removed. This applies to PR, push, scheduled and manual hosted runs. Other HustlX repositories retain their own policies.

Run `bash scripts/check.sh` on macOS or `./scripts/check.ps1` in Windows PowerShell. The checker stops on the first failure and runs formatting, workspace tests, Clippy, Python release safety tests, a native debug build and a packaging dry-run. It never dispatches GitHub jobs, builds optimized distribution packages or publishes. Dependencies may need an initial download; checks execute on the local host.

Feature branches, develop PRs, local verification and manual platform release evidence remain required. Record actual commands and the host OS. A macOS run does not prove Windows support.

Version preparation and native packaging remain available locally. The existing Python release helper and historical automation evidence are retained for future review; they do not authorize dispatch or publication. Resume hosted CI/CD only after an explicit user decision, with a review of runner cost, workflows, branch requirements and release permissions. Do not restore workflows merely to satisfy generic agent guidance.
