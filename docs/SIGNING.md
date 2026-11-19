# Assinatura de código com o SignPath

Por que: o Smart App Control do Windows 11 bloqueia programas sem assinatura digital (não tem "executar assim mesmo").
O SignPath Foundation assina projetos de código aberto de graça. O instalador assinado é o **MSI**: o SignPath assina o
`lobby-companion.exe` dentro dele e depois o MSI. O MSI não tem desinstalador próprio, então nada fica sem assinatura.
(O instalador NSIS não pode ser assinado por dentro e ficaria com o `uninstall.exe` sem assinatura.)

O que já está pronto no repositório:
- [`CODE_SIGNING_POLICY.md`](../CODE_SIGNING_POLICY.md): texto obrigatório, papéis do time e política de privacidade;
- link "Code signing policy" no README (página inicial) e nas notas de release (página de download);
- [`.github/workflows/release.yml`](../.github/workflows/release.yml): manda o MSI ao SignPath, espera, confere as assinaturas e anexa o MSI assinado ao rascunho de release;
- [`.signpath/artifact-configuration-msi.xml`](../.signpath/artifact-configuration-msi.xml): a configuração de artefato para colar no SignPath.

Enquanto a variável `SIGNPATH_ORGANIZATION_ID` não existir, o workflow gera os instaladores sem assinatura, como antes.

## Passo a passo (só você pode fazer: são contas e aprovações suas)

1. **Autenticação em dois fatores no GitHub** (obrigatória para o SignPath): https://github.com/settings/security.
2. **Candidatar o projeto**: https://signpath.org/apply. Informe o repositório (`https://github.com/nickmaglowsch/lobby-companion`),
   a licença (MIT), a página de download (`https://github.com/nickmaglowsch/lobby-companion/releases`) e a política
   (`https://github.com/nickmaglowsch/lobby-companion/blob/main/CODE_SIGNING_POLICY.md`). A resposta vem por e-mail.
3. **Depois de aprovado**, no SignPath (https://app.signpath.io), com autenticação em dois fatores ligada:
   1. *Trusted Build Systems*: adicione o **GitHub.com** à organização e vincule ao projeto.
   2. Projeto com slug **`lobby-companion`** (ou outro, e crie a variável `SIGNPATH_PROJECT_SLUG` com ele).
   3. *Artifact configurations*: crie uma com slug **`msi`** e cole o conteúdo de `.signpath/artifact-configuration-msi.xml`.
   4. *Signing policies*: **`test-signing`** (certificado de teste, sem aprovação) e **`release-signing`** (certificado da Foundation, você como aprovador).
   5. Crie um usuário de CI (*submitter* das duas políticas) e gere um **API token** para ele.
   6. Anote o **Organization ID** (em *Settings* da organização).
4. **No GitHub** (Settings → Secrets and variables → Actions):
   - *Secret* `SIGNPATH_API_TOKEN` = o token do passo 3.5;
   - *Variable* `SIGNPATH_ORGANIZATION_ID` = o ID do passo 3.6.
5. **Teste**: Actions → release → *Run workflow*. Usa `test-signing`; o passo "Confere as assinaturas" lista cada arquivo
   e o certificado (de teste, por isso o Windows ainda não confia nele). Se o SignPath não achar o exe dentro do MSI,
   ajuste o `path` do `<pe-file>` na configuração `msi`.
6. **Release**: suba a versão em `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` e `package.json`, crie e envie a tag
   (`git tag -a v0.1.1 -m "…"` e `git push origin v0.1.1`). O workflow pede a assinatura com `release-signing`:
   **aprove no SignPath** (o workflow espera até 1 hora), confira o rascunho em *Releases* e publique.

## Limites
- O certificado é emitido para a **SignPath Foundation**: é esse nome que o Windows mostra como editor.
- Mesmo assinado, o Smart App Control também olha a reputação do arquivo; os primeiros downloads de uma versão nova ainda podem ser barrados por um tempo.
- Toda release precisa da sua aprovação manual no SignPath (regra da Foundation).
