# Funcionalidade 005: popup de volume estável da Kbar

Status: proposta de mudança. Esta spec define o comportamento desejado para o
popup de volume sem autorizar, por si só, a troca do backend de áudio ou uma
alteração ampla da arquitetura da Kbar.

## Contexto

O módulo de volume da Kbar exibe um ícone e a porcentagem atual na barra
superior. Um clique abre um painel compacto com o controle de volume, mute e
as saídas de áudio disponíveis; a roda do mouse ajusta o volume e o clique do
meio alterna o mute. A barra também possui um calendário e os dois painéis
devem continuar mutuamente exclusivos.

O estado de áudio chega de forma assíncrona pelo caminho de áudio existente. O
controle local já precisa lidar com o intervalo entre um gesto do usuário, o
envio do comando e a leitura posterior do estado real.

## Problema

O popup atual compartilha o lifecycle e a árvore da superfície principal da
Kbar. O código local controla o teclado da barra quando qualquer popover abre,
mas o volume não possui tratamento explícito equivalente ao do calendário para
Escape. Essa combinação deixa o contrato de foco, fechamento e geometria
dependente de comportamentos implícitos do GTK.

O fluxo de áudio também é assíncrono: uma leitura atrasada ou otimista pode
chegar depois de uma nova intenção local. Durante a interação, o valor escolhido
pelo usuário precisa permanecer visualmente estável; fora dela, alterações
externas precisam continuar aparecendo sem flicker ou perda de convergência.

A largura própria do módulo já é limitada por tokens, request de tamanho, CSS e
um label de quatro caracteres. Não há evidência suficiente para atribuir uma
alteração visual a um mecanismo específico de popup. A mudança deve,
portanto, estabelecer e verificar a estabilidade da geometria, sem atribuir uma
causa não demonstrada.

## Comportamento desejado

### Abrir, fechar e foco

1. Um clique primário no módulo de volume DEVE abrir o popup e solicitar a
   atualização da lista de saídas.
2. A abertura repetida DEVE manter apenas um popup visível e uma única reação
   observável a cada ação do usuário.
3. O popup DEVE fechar por clique fora dele, por Escape e quando outro popup da
   Kbar for aberto.
4. Fechar e abrir novamente DEVE restaurar o popup em estado utilizável; ao
   abrir ou reabrir, o primeiro controle interativo do popup DEVE receber foco.
   Ao fechar, o foco do popup DEVE ser liberado.
5. Abrir o calendário DEVE fechar o popup de volume; abrir o volume DEVE fechar
   o calendário. Um fechamento atrasado de uma superfície não DEVE fechar nem
   limpar o estado do outro popup.
6. O teclado necessário para operar o popup DEVE pertencer ao popup enquanto
   ele estiver aberto. Depois do fechamento, nenhuma superfície DEVE ficar com
   foco ou modo de teclado preso indevidamente.
7. A abertura, o fechamento, a atualização do conteúdo ou a mudança de foco do
   popup NÃO DEVEM alterar a largura, a altura ou a posição alocada do módulo
   de volume na barra.
8. O popup DEVE permanecer associado ao monitor da barra e aparecer abaixo ou
   junto à posição correspondente do módulo, respeitando as margens e a
   geometria do tema.

### Slider e sincronização

1. Um clique direto no trilho do slider DEVE alterar o volume para a posição
   indicada pelo ponteiro.
2. Um drag lento, rápido ou cancelado DEVE atualizar a posição visual sem
   deixar a interação presa ou enviar um valor final diferente do último valor
   escolhido pelo usuário.
3. Enquanto o usuário estiver interagindo, o valor local DEVE ter prioridade
   visual sobre uma atualização externa atrasada.
4. Uma atualização externa que chegue durante o drag NÃO DEVE fazer o slider,
   os percentuais ou o ícone recuarem para um valor antigo.
5. Ao terminar a interação, o valor final DEVE ser enviado, a UI DEVE aguardar
   uma leitura de confirmação limitada e, depois da confirmação, o backend DEVE
   voltar a ser a autoridade.
6. Um snapshot conhecido como anterior à intenção local mais recente NÃO DEVE
   limpar a intenção local nem produzir flicker. Uma confirmação pertencente a
   uma interação anterior NÃO DEVE concluir uma interação posterior.
7. Fora de uma interação ou confirmação pendente, alterações externas de
   volume DEVEM atualizar o slider, o percentual da barra e o percentual do
   popup.
8. Se o backend estiver indisponível ou não fornecer um volume válido, a UI
   DEVE mostrar o estado indisponível já suportado e não DEVE bloquear a barra.
9. O intervalo de envio DEVE continuar compatível com o custo dos subprocessos
   atuais. Esta feature NÃO DEVE transformar cada evento de ponteiro em um novo
   processo nem adotar um intervalo de um frame como requisito.

### Mute, scroll e saídas

1. O botão de mute do popup e o clique do meio no módulo DEVEM continuar
   alternando o mute.
2. O ícone e os controles de mute DEVEM refletir uma alteração externa de mute
   assim que o worker publicar um estado válido.
3. Scroll para cima no módulo DEVE aumentar o volume em cinco pontos
   percentuais; scroll para baixo DEVE diminuir em cinco pontos percentuais.
4. O popup DEVE continuar exibindo a porcentagem, o estado de mute e as saídas
   disponíveis. A saída padrão DEVE permanecer identificável.
5. Selecionar uma saída DEVE continuar alterando a saída padrão pelo caminho de
   áudio existente.
6. Uma mudança somente de volume ou mute NÃO DEVE reconstruir visualmente a
   lista de saídas. Uma mudança real de identidade, nome, ordem ou saída padrão
   DEVE ser refletida corretamente, sem deixar uma seleção antiga ativa.

## Compatibilidade

A implementação DEVE preservar:

- Rust estável, GTK4, `gtk4-layer-shell` e o limite atual de `apps/kbar`;
- a superfície superior, o namespace e a seleção de monitor atuais da Kbar;
- os tokens, templates e CSS gerados do sistema de tema;
- o módulo de volume, seu ícone, percentual, mute, slider e lista de saídas;
- clique primário para abrir, clique do meio para mute e scroll de cinco pontos;
- limites de `0..=100`, clique direto, drag lento, drag rápido e cancelamento;
- a prioridade do valor local durante a interação e a autoridade do backend
  depois da sincronização;
- atualizações externas com o popup aberto ou fechado;
- execução dos comandos de áudio sem shell, com limites de timeout e tratamento
  best effort;
- coalescing de `Set` contíguos e a ordem relativa a mute, troca de saída e
  sincronização;
- a saída padrão, o fallback de uma única saída e a seleção por ID;
- a exclusividade entre Volume e Calendar e a proteção contra fechamentos
  atrasados;
- a largura reservada do módulo, o focus/hover atual e o rendering vindo da
  fonte canônica do tema;
- a possibilidade de a UI continuar funcionando quando `wpctl` ou uma fonte
  de estado estiver ausente.

## Fora de escopo

Esta feature NÃO inclui:

- mixer por aplicação ou controle de streams individuais;
- microfone, fontes de entrada ou uma política completa de Bluetooth;
- migração total para PipeWire, WirePlumber, `libpulse-binding` ou outro
  backend persistente;
- um OSD completo para scroll e hotkeys;
- outputs recolhíveis, `Revealer` ou redesign visual amplo do popup;
- troca de toolkit, QML, Qt, rewrite ou nova arquitetura global da Kbar;
- alteração dos identificadores Niri existentes ou da configuração gerada;
- uma política pública de configuração de outputs, polling ou múltiplas Kbars.

Uma evolução de backend persistente pode ser especificada posteriormente como
`006-audio-service`. A separação entre feedback rápido e painel completo pode
ser especificada posteriormente como `007-volume-osd`.

## Critérios de aceite

| ID | Critério observável | Evidência |
| --- | --- | --- |
| AC-1 | Dado que a Kbar está visível, quando o popup é aberto ou fechado, então a largura, a altura e a posição do módulo de volume permanecem inalteradas. | Manual em Wayland/Niri; testes determinísticos para helpers de geometria quando existirem. |
| AC-2 | Dado que o popup é aberto várias vezes, então há apenas um popup visível, cada ação produz uma única reação observável e o popup pode ser fechado e reaberto. | Testes de lifecycle que não dependam de renderização; manual Wayland/Niri. |
| AC-3 | Dado que Volume está aberto, quando Calendar é aberto, então Volume fecha; o inverso também funciona e um fechamento atrasado não afeta o popup novo. | Testes unitários do estado do coordinator; manual com os dois popups. |
| AC-4 | Dado que Volume está aberto, quando Escape ou um clique fora ocorre, então somente Volume fecha e o keyboard/foco não fica preso. | Manual Wayland/Niri; teste de transições lógicas sem GTK real. |
| AC-5 | Dado que o usuário clica diretamente no slider ou faz drag lento, rápido ou cancelado, então o valor final escolhido é enviado e a interação não fica presa. | Testes unitários do estado e da coalescência; manual em Wayland/Niri. |
| AC-6 | Dado que o usuário está interagindo, quando chega um snapshot externo atrasado, então o valor visual local não recua para esse snapshot. | Testes unitários da decisão de autoridade; manual durante drag com `wpctl`. |
| AC-7 | Dado que a interação terminou, quando chega a confirmação correspondente, então o backend reassume a autoridade; uma confirmação antiga não conclui uma interação nova. | Testes unitários com tokens/gerações; manual com drag consecutivo. |
| AC-8 | Dado que o popup está aberto ou fechado, quando o volume ou mute muda externamente, então a barra e o popup convergem para o estado publicado sem bloquear. | Testes do worker/parser/agregador; manual com `wpctl`. |
| AC-9 | Dado que a lista de saídas não mudou estruturalmente, quando chega um snapshot repetido ou muda apenas volume/mute, então a seleção, o foco e a aparência das opções não piscam nem são perdidos; mudanças reais de saída padrão ou identidade são refletidas. | Testes unitários de diff; manual observando seleção e estabilidade visual. |
| AC-10 | Dado que o usuário usa mute, scroll ou troca de saída, então os incrementos, o estado de mute e a saída padrão continuam funcionando. | Testes de comandos/parsing existentes; manual Wayland/Niri/PipeWire. |
| AC-11 | Dado que `wpctl` está ausente, falha ou demora, então a UI mostra estado indisponível ou mantém o último estado válido e permanece responsiva. | Testes de parsing/timeout; manual com backend indisponível quando possível. |
| AC-12 | Dado que a feature é aplicada, então a aparência do módulo e do popup continua alinhada aos tokens visuais atuais, sem introduzir uma paleta, tipografia ou geometria paralela. | Inspeção manual e `cargo run -p kshell-theme-gen -- --check` quando houver mudança de template. |

### Gates automatizados

Quando houver código alterado, devem passar os gates definidos em `AGENTS.md`:

```sh
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
cargo check --workspace
cargo run -p kshell-theme-gen -- --check
```

### Validação manual Wayland/Niri

Os critérios AC-1 a AC-11 que envolvem GTK, layer-shell, foco, geometria,
subprocessos ou compositor exigem uma sessão Wayland/Niri adequada. A execução
manual deve cobrir abrir, fechar, reabrir, Escape, click-outside, Calendar,
estabilidade do módulo, drag lento/rápido, clique direto, scroll, mute,
alterações externas, alteração durante drag, troca de saída e backend
indisponível. O resultado deve registrar separadamente o que foi automatizado e
o que dependeu da sessão.
