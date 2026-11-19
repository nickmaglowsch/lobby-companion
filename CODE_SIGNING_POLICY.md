# Code signing policy

Free code signing provided by [SignPath.io](https://about.signpath.io/), certificate by [SignPath Foundation](https://signpath.org/).

## What is signed

Only the Windows installer (`.msi`) published in the [Releases](https://github.com/nickmaglowsch/lobby-companion/releases) of this repository, and the application executable inside it (`lobby-companion.exe`).
Every signed file is built by GitHub Actions from the source code in this repository ([`.github/workflows/release.yml`](.github/workflows/release.yml)), on GitHub-hosted runners, without manual steps.
The repository contains no proprietary components; third-party dependencies are open source (Rust crates and npm packages listed in `Cargo.lock` and `package-lock.json`).

## Team roles

| Role | Members |
|---|---|
| Committers and reviewers | [nickmaglowsch](https://github.com/nickmaglowsch) |
| Approvers | [nickmaglowsch](https://github.com/nickmaglowsch) |

Every release signing request is approved manually by an approver in SignPath. All team members use multi-factor authentication on GitHub and SignPath.

## Privacy policy

This program will not transfer any information to other networked systems unless specifically requested by the user or the person installing or operating it, with the following exceptions, which are the program's core function:

- **League of Legends client, on the same computer** (`127.0.0.1`): the program reads the champion select session and writes the rune page, summoner spells and item set the user asked it to import. This traffic never leaves the computer.
- **lolalytics.com** (`a1.lolalytics.com`): when the user locks or searches for a champion, the program requests that champion's build statistics. The request contains only the champion, lane, rank tier, region and patch filters. No account name, Riot ID, PUUID, email or any other personal or device identifier is sent. Responses are cached locally for 6 hours.

The program has no telemetry, analytics, ads or auto-update, and stores its settings and cache only on the local computer.

---

# Política de assinatura de código (pt-BR)

Assinatura de código gratuita fornecida por [SignPath.io](https://about.signpath.io/), certificado da [SignPath Foundation](https://signpath.org/).

Só o instalador `.msi` das *Releases* deste repositório (e o `lobby-companion.exe` dentro dele) é assinado, sempre gerado pelo GitHub Actions a partir deste código, e cada assinatura de release é aprovada manualmente no SignPath. Papéis: [nickmaglowsch](https://github.com/nickmaglowsch) é autor, revisor e aprovador.

**Privacidade:** o programa só se comunica com o cliente do League no próprio computador (`127.0.0.1`) e com o lolalytics.com, mandando apenas campeão, lane, elo, região e patch. Nenhum nome, Riot ID, PUUID, e-mail ou identificador do computador é enviado. Sem telemetria, propaganda ou atualização automática.
