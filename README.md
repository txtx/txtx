<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/txtx/txtx/main/doc/assets/dark-theme.png">
  <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/txtx/txtx/main/doc/assets/light-theme.png">
  <img alt="Txtx" width="190" height="92" style="max-width: 100%;">
</picture>

**BUILD CONFIDENCE WITH WEB3 RUNBOOKS**
______________________________________________________________________

<!-- [![License](https://img.shields.io/badge/License-Apache2-blue)](/LICENSE) -->
<!-- [![chat](https://img.shields.io/discord/1179456777406922913?logo=discord&logoColor=white&color=%235765F2)](https://discord.gg/rqXmWsn2ja) -->
</div>

## Latest News 🔥

- Added support for deployments of Solana programs - including support for other SVM chains (Eclipse, etc.)
- Added support for EVM deterministic deployments via CREATE2
- Added support for Zero Knowledge Proof based transaction flows with [Sp1 addon](https://github.com/txtx/txtx/pull/140)
- The 1st runbook ever executed on Mainnet moved [$2.5M](https://explorer.hiro.so/txid/70f0b5d238fae566756526678939307b18673bd864b6d74eb5f050b3f8226855?chain=mainnet&api=https://api.hiro.so)!
- Added support for distributed multisig ceremonies in Stacks addon.

## What is Txtx?

Have you ever tried to deploy some open source Smart contracts on a local devnet, just to get stuck on the first steps of understanding how the deployment should be executed? Have you lost hundreds, thousands or millions of tokens because your master private key got compromised after copy/pasting in your deployment scripts, or missed some contracts initializations making your deployments vulnerable and flawed?

**Txtx** turns the stress, pain and complexity of Smart Contract Infrastructure management into a secure, reproducible and proficient developer experience. 

Txtx introduces **Smart Contract Runbooks** to assist developers to deploy and operate on their Smart Contracts / Solana Programs / Bitcoin Scripts.

**Smart Contract Runbooks** are the blueprint for Engineering Excellence, setting the gold standard for Web3 Infrastructures. 

Txtx is to Web3 what Hashicorp Terraform is to cloud infrastructure management: thanks to infrastructure as code, developers now have the ability to enhance their Web3 operations by leveling up security, composability, and reproducibility.

With Txtx we're introducing:

- A declarative language, inspired from Hashicorp Terraform, tailored for describing your Web3 deployments and operations

- A runtime allowing you to perform stateful executions: the state of your previous executions is used against the current state of your files, allowing to only execute the updates.

- An optional Web UI running on your machine, guiding you through the execution of your runbooks, facilitating use of web wallets, automating wallet provisionning, distributed multisig ceremonies, etc.

<div align="center">
  <picture>
    <source srcset="https://raw.githubusercontent.com/txtx/txtx/main/doc/assets/supervisor.png">
    <img alt="Txtx - Supervisor" style="max-width: 60%;">
  </picture>
</div>

> [!IMPORTANT]
> Txtx is currently in beta: We’re still testing and refining the platform. Please use it for testing purposes only. Your feedback is welcome! 🙌

## Txtx in action: 101 Demos 

<a href="https://www.youtube.com/playlist?list=PL0FMgRjJMRzMcA23x6y_1lkxXUmuqOlKu">
  <picture>
    <source srcset="https://raw.githubusercontent.com/txtx/txtx/main/doc/assets/youtube.png">
    <img alt="Txtx - Web3 Runbook - 101 series" style="max-width: 100%;">
  </picture>
</a>

## Declarative Automation

In a composable context (and Web3 is inherently composable), declarative deployments and operations are essential.

As infrastructure complexity increases, we must be able to test and reproduce deployments in clean, isolated environments. Additionally, we need the capability to update, migrate, add, or remove components without going offline or putting funds, tokens, and assets at risk.

Txtx is purpose-built for blockchain operations, drawing on the best practices that were developed over time in cloud infrastructure management.

## Focus on Security

Every year, between $500M and $1B are lost due to compromised private keys. As developers, we often do exactly what we tell our users not to do: leaving our private keys too accessible for easy copying and pasting during deployments and operations.

Txtx is eliminating these risky practices by introducing script execution in the browser, where execution interactively prompts for signatures. These signatures can be securely provided using your web wallet, hardware wallet, or even multisig ceremonies.

In addition to boosting security, the Web Supervisor UI helps developers by identifying common pitfalls and guiding them toward safer practices.

Finally, the declarative nature of Txtx runbooks, combined with a state-aware runtime, ensures that smart contracts are always configured as intended. This class of issues has historically led to some of the [largest hacks](https://www.theverge.com/2022/2/3/22916111/wormhole-hack-github-error-325-million-theft-ethereum-solana) in Web3 history.


## Quick Start

The txtx CLI tool can be installed via our install script or through a manual build.

### Install on macOS (Homebrew)

To install txtx on a macOS system using Homebrew, open a terminal and run:

```bash
brew install txtx/taps/txtx
```

Other installation options are available and described in our [doc website](https://docs.txtx.sh/install).

## Documentation

### Local Documentation
- [**User Guides**](docs/user/) - LSP setup, Doctor command usage
- [**Developer Docs**](docs/developer/) - Architecture, testing, contributing
- [**All Documentation**](docs/) - Complete documentation index

### Online Resources
- Documentation: https://docs.txtx.sh
- Cases Study: https://txtx.sh/blog
- Demos and Screencasts: https://www.youtube.com/@runtxtx
