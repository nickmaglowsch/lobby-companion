# Lobby Companion

Companion **leve** de League of Legends para a **seleção de campeões**, com dados do [lolalytics](https://lolalytics.com).
Quando você trava o campeão, ele importa para o cliente:

- a **página de runas** (a mais jogada ou a de maior win rate);
- os **feitiços de invocador** (mantendo o Flash na tecla que você usa);
- um **conjunto de itens** com o **win rate de cada item no título do bloco** (aparece na loja dentro da partida):
  `Início · WR 51,8%`, `Botas · WR 55,4% / 51,7%`, `1º item · WR 53,3% / 53,1%`… até o 5º item.

A janela mostra o build completo: runas, feitiços, ordem de habilidades, inícios, botas e uma tabela de itens por posição (1º, 2º, 3º…) com WR, pick rate e partidas. Trocar a runa, a lane ou um filtro na janela já atualiza o cliente.

- **Confronto:** escolha o campeão do outro lado (retrato do time inimigo ou busca) para ver e importar runas, feitiços e itens daquele confronto (dados globais do lolalytics, amostra menor).
- **Sugestão de bans:** pelos campeões escolhidos ou declarados do seu time, os inimigos que mais derrubam o win rate de vocês, com o motivo de cada um.

**Sem overlay.** A janela abre sozinha na seleção de campeões e **fecha quando a partida começa**; fora disso, só um ícone na bandeja (sem o WebView, só o processo Rust ouvindo o WebSocket do cliente).

> Não afiliado à Riot Games nem ao lolalytics. O lolalytics não tem API pública: o app usa o mesmo endpoint JSON que o site usa, com cache de 6 h e no máximo uma requisição por segundo, e sempre linka a página original.

**[Code signing policy](CODE_SIGNING_POLICY.md)** · Free code signing provided by [SignPath.io](https://about.signpath.io/), certificate by [SignPath Foundation](https://signpath.org/) (em andamento: veja [docs/SIGNING.md](docs/SIGNING.md)). Privacidade: o app só fala com o cliente do League no seu computador e com o lolalytics (campeão, lane e filtros; nada pessoal).

## Instalar e usar

1. Baixe o instalador em *Releases* (ou compile, abaixo) e abra o app. Prefira o **`.msi`** quando houver versão assinada: é ele que o Smart App Control aceita.
2. Abra o cliente do League. O ponto verde no topo indica que o app achou o cliente.
3. Entre numa fila. Na seleção de campeões, trave o campeão: runas, feitiços e itens entram sozinhos.
4. Fora da seleção, use a busca para ver o build de qualquer campeão/lane.

Configurações (na própria janela): elo (padrão **Esmeralda+**), região (padrão **global**), período (patch atual / 7 / 30 dias), o que importar, amostra mínima por item, tecla do Flash, substituir a página atual quando não houver espaço para uma nova.

O app **nunca** escolhe, bane, aceita fila nem lê nomes de outros jogadores. Ele só escreve em:
`/lol-perks/v1/pages`, `/lol-perks/v1/currentpage`, `/lol-item-sets/v1/item-sets/{id}/sets` e `/lol-champ-select/v1/session/my-selection`.
A página de runas criada se chama `LC: <Campeão> <Lane>`; o conjunto de itens, `LC: …`. Os seus conjuntos e páginas ficam intactos.

## Desenvolvimento

Pré-requisitos (Windows): [Rust](https://rustup.rs), Visual Studio Build Tools com "Desenvolvimento para desktop com C++", Node 20+.

```bash
npm install
npm run tauri dev        # app com recarga
npm run dev              # só a interface no navegador, com dados sintéticos
npm run tauri build      # instalador NSIS em src-tauri/target/release/bundle/
```

Diagnóstico pelo terminal (sem abrir janela):

```bash
cd src-tauri
cargo run -- --diag                 # cliente: porta, fase, inventário de runas, seleção atual
cargo run -- --lola chogath jungle  # build do lolalytics resumido
cargo test
```

Estrutura:

| Caminho | O quê |
|---|---|
| `src-tauri/src/lcu/` | credenciais (lockfile), cliente HTTP com lista de escritas permitidas, WebSocket (WAMP) |
| `src-tauri/src/lola/` | cliente do lolalytics (cache, ritmo) e parser do `build-full` |
| `src-tauri/src/import/` | runas, conjunto de itens, feitiços |
| `src-tauri/src/champselect.rs` | leitura da seleção (só o próprio jogador) |
| `src-tauri/src/app.rs` | estado e laço de eventos |
| `src/` | interface (SvelteKit + TypeScript) |

## Licença

MIT. League of Legends é marca da Riot Games, Inc. Dados de builds: lolalytics.com.
