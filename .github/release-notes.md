## Novidades
{{CHANGES}}

## Atualizar
- **Da 0.4.0 ou mais nova:** aparece um aviso no app. Clique em **Atualizar agora** (fora da seleção de campeões).
- **Da 0.3.x ou anterior:** baixe e instale o arquivo abaixo uma vez; as versões antigas não têm o atualizador.

## Instalar
1. Baixe `{{EXE}}` abaixo e execute. Ele instala só para o seu usuário e pode ir por cima da versão anterior, mantendo as configurações.
2. O instalador **não é assinado digitalmente** (Windows). O SmartScreen mostra "editor desconhecido": clique em **Mais informações → Executar assim mesmo**. Se o Smart App Control estiver ligado, ele pode bloquear o app.
3. Abra o cliente do League. O ponto verde no topo indica que o app achou o cliente.

SHA-256 do instalador: `{{SHA}}`
(gerado pelo GitHub Actions, [execução {{RUN_ID}}]({{RUN_URL}}), commit `{{COMMIT}}`).

## Assinatura digital
O instalador **não é assinado** com certificado do Windows. Confira o SHA-256 acima: ele bate com o arquivo que o GitHub Actions gerou a partir do código desta tag.

A atualização pelo app usa a assinatura do atualizador do Tauri: o app só roda um instalador baixado se ele foi assinado com a chave do projeto.

Privacidade: o app só se comunica com o cliente do League no seu computador, com o lolalytics.com (campeões, lanes e filtros) e com o GitHub, para saber se há versão nova (pode desligar nas configurações). Nada pessoal é enviado.

## Avisos
- Não afiliado à Riot Games nem ao lolalytics.
- As sugestões de pick e ban, os counters e as sinergias são tendência estatística do lolalytics, não leitura do outro time.
- Versão lançada automaticamente a cada mudança mesclada; relate problemas em *Issues*.
