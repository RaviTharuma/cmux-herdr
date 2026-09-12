# Disclaimer

**Read this before you install, redistribute, or rely on cmux-herdr.**

This project is **free, unpaid, open-source software** published for
convenience. Using it is voluntary. By cloning, downloading, installing,
running, forking, or building on it, **you accept the terms below and the
[MIT License](LICENSE).**

This file is **not legal advice**. Absolute immunity from every claim in
every country is not something a markdown file (or any OSS license) can
guarantee. What this repo does is use the standard MIT bargain and state
risk allocation as clearly as possible: **permission to use free code,
zero paid obligation, maximum disclaimer of warranty and liability.**
If you need advice for your situation, talk to a lawyer in your
jurisdiction.

## Independence

**cmux-herdr** is an independent, community-maintained plugin. It is **not**
an official product of [manaflow-ai/cmux](https://github.com/manaflow-ai/cmux),
[herdrdev/herdr](https://github.com/herdrdev/herdr), their authors, or their
employers. No endorsement, partnership, agency, joint venture, employment,
or consumer-seller relationship is created by this repository, its docs,
releases, Actions, bots, or issues.

## No warranty (as is)

THE SOFTWARE IS PROVIDED **“AS IS”** AND **“AS AVAILABLE”**, WITHOUT WARRANTY
OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE, TITLE, NON-INFRINGEMENT,
ACCURACY, RELIABILITY, SECURITY, OR AVAILABILITY — matching and restating the
[MIT License](LICENSE).

In plain language:

- Nobody promises it will work for you, keep working, or be suitable for
  any particular job.
- Nobody promises it is free of bugs, vulnerabilities, supply-chain
  compromise, or malicious or negligent contributor changes.
- Nobody promises it will not lose data, scramble sessions, write status to
  the wrong workspace, burn API/compute credits, or interact badly with
  cmux / Herdr / your OS / your cloud accounts.
- Docs, issue replies, PR reviews, CI checks, CODEOWNERS pings, Dependabot
  bumps, release notes, and agent-generated patches are **informational
  process**, not warranties, certifications, or professional advice.

## Limitation of liability (including supply chain, credits, bad PRs)

TO THE MAXIMUM EXTENT PERMITTED BY APPLICABLE LAW, THE AUTHORS, COPYRIGHT
HOLDERS, MAINTAINERS, AND CONTRIBUTORS SHALL **NOT** BE LIABLE FOR ANY
CLAIM, DAMAGES, OR OTHER LIABILITY — whether in contract, tort (including
negligence), strict liability, product liability, or otherwise — arising
from, out of, or in connection with the software or the use or other
dealings in the software.

That includes, **without limitation**, claims or losses from:

| Category | Examples (non-exhaustive) |
|---|---|
| **Runtime / data** | Data loss; broken or leaked sessions; wrong pills/mirrors/focus; downtime; lost profits; business interruption |
| **Supply chain** | Compromised dependencies; malicious or typosquat packages; poisoned GitHub Actions / CI; tampered or mirrored release artifacts; compromised maintainer or contributor accounts; dependency confusion |
| **Contributor / merge risk** | Malicious, negligent, or accidental harmful code that was proposed, reviewed, auto-suggested, or **merged** into this repository |
| **Credits & resources** | Over-use or unexpected spend of API keys, LLM/agent credits, cloud quotas, CI minutes, bandwidth, electricity, or third-party paid services triggered while running, testing, or integrating this software |
| **Privacy / ops** | Incidents caused by how *you* run tools, share logs, or wire secrets into agents or CI |
| **Damages types** | Direct, indirect, incidental, special, consequential, exemplary, or punitive damages — even if advised they were possible |

If your jurisdiction does not allow some exclusions, those exclusions apply
only to the extent disallowed; everything else still applies. Where liability
cannot be fully excluded, it is limited to the greater of (a) **US $0**
(this software is free of charge) or (b) the minimum amount required by
mandatory law.

**There is no indemnity from the author.** Publishing free MIT code is not
an agreement to defend, indemnify, or hold you harmless.

## Supply chain and third-party tooling

You are solely responsible for:

- Verifying release checksums / signatures when you install binaries
- Pinning and auditing dependencies and GitHub Actions you run
- Treating CI green checks as **non-authoritative** (they can be bypassed,
  poisoned, or incomplete)
- Isolating secrets; never pasting production credentials into issues, PRs,
  or agent chats linked to this repo

The authors are **not** responsible for attacks or failures in npm/crates
registries, GitHub, runners, mirrors, CDNs, `cmux`, `herdr`, or any other
third-party system this project touches.

## Contributor and merge risk

Anyone can open a pull request. Review, CODEOWNERS requests, bots, and CI
**reduce** risk; they do **not** eliminate it and they do **not** create a
guarantee that merged code is safe, lawful, or non-malicious.

If harmful code is contributed and merged—accidentally or on purpose—**you
still assume the risk of running that software**. Maintainers are not
liable for failing to catch it. See [CONTRIBUTING.md](CONTRIBUTING.md) and
[GOVERNANCE.md](GOVERNANCE.md).

## Credits, quotas, and paid third-party usage

This plugin does not sell credits. If you (or an agent acting for you)
connect API keys, cloud accounts, CI billing, or LLM credits to workflows
involving this repo, **any over-usage, runaway loops, or unexpected bills
are your responsibility**. The authors have no visibility into or control
over your keys, budgets, or spend limits.

## Assumption of risk

You alone decide whether to install and run this plugin on your machines.
You alone are responsible for backups, testing, rollbacks, employer/client
policy, local law, secret redaction, and how you combine this plugin with
other tools.

If something goes wrong, **that risk is yours**, not the author’s, for free
code published under MIT.

## No support, no SLA, no duty of care

There is **no support desk**, paid plan, service contract, uptime promise,
security guarantee, or response-time SLA. Opening an issue or PR does
**not** create an obligation to fix, reply, audit, or maintain anything.
Maintainers may ignore, close, or delete issues and PRs at their
discretion. See [ISSUE_REPORTING.md](ISSUE_REPORTING.md).

## What this plugin is (and is not)

| This plugin **is** | This plugin **is not** |
|---|---|
| A user-installed cmux plugin | A patch to `cmux.app` or Herdr |
| External pane viewers + status pills | Native Herdr UI / Ghostty PTY takeover |
| A bridge that shells out to `cmux` and `herdr` | A replacement for either CLI |
| Best-effort local tooling | A cloud service, SaaS, or hosted product |
| Free MIT-licensed software | A paid product, insured service, or certified secure supply chain |

Shipped versus planned capabilities: README
[capability matrix](README.md#capability-matrix) and [OPEN.md](OPEN.md).
Notes under `docs/upstream/` are **targets**, not guarantees.

## Third-party names

"cmux", "Herdr", "Ghostty", "tmux", and related marks belong to their
owners. Use here is descriptive only and implies no endorsement or
affiliation.

## Contributions

By contributing (issue, PR, comment, patch, or bot-submitted change), you
license your contribution under the same [MIT License](LICENSE), **as is**,
with no extra warranty. You represent that you have the right to submit it.
Contributions do not create employment, partnership, indemnity, or a
support contract.

## Security and privacy

- No cmux-herdr cloud backend, accounts, or first-party telemetry.
  See [PRIVACY.md](PRIVACY.md).
- Local state under `$XDG_STATE_HOME/cmux-herdr/` stays on your machine
  unless *you* copy it elsewhere.
- Report vulnerabilities privately per [SECURITY.md](SECURITY.md).

## Upstream bugs

Bugs in cmux or Herdr belong on those trackers. Routing:
[ISSUE_REPORTING.md](ISSUE_REPORTING.md).

## Governing license text

If anything here conflicts with the [MIT License](LICENSE), the license
controls for the grant of rights; this file clarifies intent, scope, and
risk allocation. Publishing this repository under MIT is an offer of
**permission to use free software without warranty**, not an offer of
services, indemnity, insurance, or a certified secure supply chain.
