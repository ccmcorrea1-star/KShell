# Plano da funcionalidade 005: popup de volume estável da Kbar

Este plano descreve o delta técnico para cumprir
[`spec.md`](spec.md). A mudança permanece dentro de `apps/kbar`, usa GTK4 e
`gtk4-layer-shell`, preserva `AudioBackend`/`WpctlBackend` e não altera a
arquitetura global do workspace.

## Limites da mudança

- A barra principal continua sendo a surface superior existente, com o
  namespace, monitor, exclusive zone e tokens atuais.
- O popup de volume passa a ter uma surface própria, mas continua sendo uma
  parte da Kbar e não uma nova aplicação ou um serviço independente.
- Calendar continua com seu popup atual nesta feature; a integração entre os
  dois lifecycles será adaptada apenas no limite necessário para manter a
  exclusividade.
- A lógica determinística de estado, diff e comandos continua testável fora de
  uma sessão GTK real.
- O CSS versionado continua sendo output do theme generator. Se o novo tipo de
  surface exigir seletores diferentes, a fonte será o template, seguida de
  regeneração.

## Estratégia de uso de subagentes durante a implementação

Esta seção orienta a execução futura das tasks. Ela não define uma quantidade
fixa de subagentes nem uma coreografia por ID de task. Antes de iniciar cada
task, o agente principal deve ler a task, confirmar suas dependências, mapear o
escopo real encontrado no workspace e identificar oportunidades concretas de
delegação. Deve registrar brevemente a estratégia escolhida — delegação,
execução serial ou execução local sem subagentes — antes de começar o trabalho.

### Quando há benefício real

Spawnar subagentes pode ser útil quando houver trabalho independente e
verificável, por exemplo:

- investigação de módulos independentes ou análise do código existente antes
  de alterar uma superfície compartilhada;
- pesquisa de implementação externa, documentação primária ou alternativas
  técnicas específicas;
- análise separada de UI/layer-shell, backend de áudio e estado determinístico;
- debugging com hipóteses independentes que possam ser verificadas sem
  compartilhar um write-set;
- revisão de uma mudança estrutural depois que a implementação estiver pronta;
- trabalho paralelo em arquivos que não se sobrepõem e cujas interfaces já
  estejam definidas;
- revisão final contra `spec.md`, `plan.md`, compatibilidade e critérios de
  aceite.

O benefício deve ser concreto: reduzir investigação, aumentar cobertura de
revisão ou permitir trabalho independente. Não spawnar por formalidade quando a
task for pequena, tiver uma única sequência causal, exigir contexto contínuo ou
puder ser concluída com segurança pelo agente principal em pouco tempo.

### Quando o paralelismo é seguro

O paralelismo é apropriado principalmente para análises somente leitura,
pesquisas independentes e verificações que não dependem umas das outras. Também
pode ser usado para implementação quando cada participante tiver um write-set
disjunto, arquivos claramente delimitados e um contrato de integração explícito
antes do início.

Antes de delegar trabalho com escrita, o agente principal deve declarar para
cada subagente:

- objetivo e resultado esperado;
- arquivos e símbolos que pode alterar;
- arquivos que não pode tocar;
- dependências e interfaces que devem permanecer estáveis;
- validações que deve executar e relatar.

Subagentes não devem editar o mesmo arquivo, o mesmo módulo de estado ou a
mesma interface compartilhada em paralelo. Se a separação não puder ser
expressa com clareza, o trabalho deve ser serializado ou permanecer com o
agente principal. Alterações geradas por templates também devem ter um único
responsável por sua fonte e regeneração.

### Quando serializar

Preferir execução serial quando:

- a task altera ownership, lifecycle, coordinator, protocolo ou uma API usada
  por várias partes;
- o resultado de uma análise define a próxima decisão de implementação;
- duas mudanças dependem do mesmo estado intermediário ou do mesmo arquivo;
- há migração incremental em que o sistema não pode permanecer com dois
  caminhos ativos;
- o problema exige reproduzir uma sequência temporal única de UI, worker e
  backend;
- a integração, o conflito potencial ou a validação final custariam mais que o
  paralelismo economizado.

Em particular, decisões sobre a surface de Volume, a exclusividade com
Calendar e o protocolo de confirmação do slider devem ser integradas pelo
agente principal antes de delegar qualquer alteração dependente delas.

### Escolha do papel do subagente

O papel deve corresponder ao risco concreto da task:

- **Análise:** mapear chamadas, lifecycle, estado, testes e efeitos de uma
  mudança; preferir saída somente leitura antes de uma refatoração estrutural.
- **Implementação:** executar uma alteração limitada a um write-set disjunto,
  com critérios de aceite e validações definidos; não decidir sozinho mudanças
  de escopo.
- **Pesquisa:** estudar uma implementação externa ou documentação primária e
  entregar caminhos, símbolos, conceitos e limites de aplicabilidade; não
  transplantar arquitetura automaticamente.
- **Revisão:** procurar regressões contra `spec.md`, `plan.md`, compatibilidade,
  erros de lifecycle, write-sets e critérios de aceite depois de uma mudança.
- **Debugging:** testar hipóteses independentes, isolando UI, backend, estado ou
  timing quando cada hipótese puder ser investigada sem alterar o mesmo código.

O agente principal deve escolher o menor conjunto de papéis necessário. Um
subagente de análise não deve iniciar implementação por conta própria; um
subagente de implementação não deve avançar para a task seguinte; e um
subagente de revisão não deve corrigir arquivos sem autorização explícita do
agente principal.

### Responsabilidade do agente principal

O agente principal permanece responsável por:

- decidir se a delegação é útil para a task real, e não para o número da task;
- fornecer contexto suficiente sem delegar decisões de escopo indefinidas;
- integrar resultados, resolver conflitos e revisar qualquer alteração
  recebida;
- verificar que os write-sets não se sobrepõem e que a fonte de verdade correta
  foi alterada;
- executar ou coordenar toda a validação automatizada e manual possível;
- comparar o resultado final com `spec.md`, `plan.md`, tasks dependentes e
  compatibilidade atual.

Subagentes não devem avançar para tasks seguintes, alterar requisitos da spec,
marcar validação manual como executada ou declarar a feature concluída. Um
resultado de subagente é evidência para integração, não substituto da revisão e
da validação do agente principal.

### Registro mínimo antes de cada task

Antes de iniciar uma task, registrar brevemente:

1. escopo e arquivos/símbolos realmente envolvidos;
2. se existe análise, pesquisa, implementação ou revisão que possa ser
   delegada sem conflito;
3. se o trabalho será paralelo ou serial e por quê;
4. write-set de cada subagente, quando houver;
5. validação que ficará sob responsabilidade do agente principal.

Se nenhuma oportunidade concreta aparecer, registrar que a task será executada
localmente e seguir sem spawnar por formalidade. A estratégia pode mudar quando
a investigação revelar um escopo diferente, desde que o agente principal
atualize o registro antes de ampliar a delegação.

## Decisão 1 — Surface layer-shell independente para o volume

### Problema

O volume atual é um popup parentado ao item da barra. A janela principal é
criada com `KeyboardMode::None`, e `PopoverCoordinator` a troca para
`OnDemand` quando qualquer popup abre. O volume não tem uma surface própria,
nem um tratamento explícito de Escape ou um lifecycle independente.

### Evidência no KShell

- `apps/kbar/src/ui/volume.rs:114-125` cria o popup atual, usa
  `set_autohide`, aplica offsets e chama `set_parent(&volume_item)`.
- `apps/kbar/src/ui/popover.rs:43-81` registra popovers GTK, fecha o anterior
  e altera o keyboard mode da janela principal.
- `apps/kbar/src/app.rs:69-98` concentra a surface layer-shell e a composição
  da barra.
- Não há código local que prove que `GtkPopover` causa reflow; o problema é a
  dependência de lifecycle/foco implícitos, não uma causa visual já demonstrada.

### Referência

Projeto: VibePanel

Arquivo: [`layer_shell_popover.rs`](https://github.com/prankstr/vibepanel/blob/92063a36833ca8eda61a02485f8bfadcc73e1d43/crates/vibepanel/src/widgets/layer_shell_popover.rs)

Tipos/funções: `LayerShellPopover`, `ensure_window_shell`,
`ensure_click_catcher`, `show_at`, `hide`, `setup_esc_handler`.

Conceito extraído: uma `ApplicationWindow` própria, configurada como
layer-shell e acompanhada por um click catcher, pode ter teclado, monitor,
margens e lifecycle sem depender da árvore da barra.

### Decisão

Criar uma `ApplicationWindow` própria para o popup de volume, com:

- layer `Top`;
- exclusive zone zero, pois o popup não deve reservar espaço da barra;
- monitor já resolvido pelo `OutputContext` local da Kbar em
  `apps/kbar/src/app.rs:156-185`;
- namespace dedicado e centralizado junto aos identificadores Niri existentes,
  por exemplo `my-shell-volume-popup`, sem reutilizar `BAR_NAMESPACE`;
- `KeyboardMode::OnDemand` somente enquanto o popup estiver ativo;
- ownership por um handle mantido pelo volume/coordinator, sem parent GTK na
  `ApplicationWindow` principal.

O click catcher será uma surface auxiliar mínima, transparente e reutilizável,
usada apenas para fechar por clique fora. A superfície do popup continuará
acima do catcher e o catcher não terá exclusive zone. A implementação deve
evitar criar uma abstração de surfaces para todas as aplicações antes de haver
uma segunda consumidora real.

### Alternativas rejeitadas

- Manter `GtkPopover` e apenas alterar offsets: não separa keyboard, foco e
  lifecycle da barra.
- Reutilizar o namespace da barra: mistura a identidade de duas surfaces.
- Criar uma nova aplicação/processo para o volume: amplia o lifecycle e quebra
  o limite atual da Kbar.
- Criar um framework genérico de popups layer-shell: não há necessidade de
  generalização antes de o Calendar também exigir uma surface independente.

### Riscos

- Uma surface fullscreen de click catcher pode interceptar o popup ou a barra
  se a ordem das surfaces e as regiões de input não forem configuradas
  corretamente.
- Namespace, monitor e keyboard mode incorretos podem tornar o popup invisível
  ou roubar teclado da Kbar.
- Um handle mantido em closure pode duplicar callbacks se a criação não for
  realmente única.

### Validação

- Testar a criação e a transição lógica sem GTK real onde possível.
- Em Wayland/Niri, abrir o volume, confirmar surface independente, foco,
  Escape, click-outside, ausência de exclusive zone e retorno do teclado após o
  fechamento.
- Confirmar que a barra continua com uma única `ApplicationWindow` principal e
  que o módulo não muda de geometria.

## Decisão 2 — Lifecycle persistente, posicionamento e dimensões

### Problema

O popup atual é criado junto com a barra, mas seu posicionamento é resolvido
pelas regras do `GtkPopover`. A nova surface precisa aparecer no monitor correto
e ser reaberta sem duplicação, sem usar dimensões zero de uma surface oculta ou
alterar a alocação do módulo.

### Evidência no KShell

- `apps/kbar/src/ui/volume.rs:118-140` usa posição `Bottom`, offset vertical
  derivado dos tokens e width request de `VOLUME_POPOVER_WIDTH`.
- `apps/kbar/src/app.rs:156-185` resolve `KSHELL_OUTPUT` para um
  `gdk::Monitor` e aplica o monitor à barra.
- `crates/theme/src/tokens.rs:70-83` define `BAR_HEIGHT`, `BAR_MARGIN`,
  `VOLUME_MODULE_WIDTH`, `VOLUME_POPOVER_WIDTH` e altura das rows.
- `apps/kbar/src/ui/status.rs:29-49,84-99` mostra que a bateria pode alterar a
  largura do grupo de status independentemente do popup; isso não deve ser
  confundido com reflow causado pela abertura.

### Referência

Projeto: VibePanel

Arquivo: [`quick_settings/window.rs`](https://github.com/prankstr/vibepanel/blob/92063a36833ca8eda61a02485f8bfadcc73e1d43/crates/vibepanel/src/widgets/quick_settings/window.rs)

Tipos/funções: `QuickSettingsWindowHandle::toggle_at`,
`QuickSettingsWindow::new`, `show_panel`, `hide_panel`, `set_anchor_position`,
`update_position`, `cached_width` e `cached_height`.

Conceito extraído: criação lazy, window persistente, estado lógico separado de
`set_visible` e cache de dimensão quando a surface oculta reporta zero.

### Decisão

O popup será criado sob demanda na primeira abertura e reutilizado até o
lifecycle da Kbar terminar. O estado lógico aberto/fechado será separado de
`visible`. Os únicos handlers registrados uma vez nesta surface serão os de
show/hide, Escape, click-outside e abertura/fechamento do conteúdo; o bridge
global de `StatusUpdate::Audio` em `apps/kbar/src/app.rs:100-144` continua sendo
a única entrada de atualizações de áudio, sem subscriptions de áudio criadas
por abertura do popup.

Para a barra atual, que fica no topo, o posicionamento será calculado a partir
de:

1. monitor resolvido pela barra;
2. retângulo alocado do módulo de volume;
3. largura/altura efetivas ou últimas dimensões válidas do popup;
4. `BAR_HEIGHT`, `BAR_MARGIN`, offset atual e margens do tema.

A surface será ancorada ao topo e à borda horizontal necessária, com margens
calculadas para que o popup acompanhe o módulo sem parent GTK. O helper deve
manter a largura de `VOLUME_POPOVER_WIDTH` e limitar o popup ao monitor. Se uma
surface oculta reportar dimensão zero, serão usadas as últimas dimensões válidas
ou o tamanho tokenizado; não será introduzida animação para resolver esse caso.

O tratamento explícito de Escape ficará na surface do volume. Click-outside
será encaminhado pelo click catcher. O volume não mudará o keyboard mode da
janela principal; a surface principal permanecerá em `None` quando somente
Volume estiver aberto.

### Alternativas rejeitadas

- Recalcular posição por tentativa a cada frame: aumenta acoplamento ao frame
  Wayland e não é necessário para um popup de tamanho conhecido.
- Manter apenas uma janela criada no startup: conserva ownership simples, mas
  não resolve o requisito de surface independente nem separa o lifecycle.
- Copiar `SurfaceHeightFreeze`, blur e animações completas do VibePanel: são
  mecanismos de UX não justificados pelo problema atual.

### Riscos

- O retângulo de um widget GTK pode não estar disponível no mesmo momento em
  que a surface é exibida.
- Margens invertidas podem posicionar o popup fora do monitor em resoluções
  pequenas.
- Subscriptions persistentes podem continuar consumindo updates quando o popup
  está oculto; isso deve ser uma decisão consciente e não uma duplicação.

### Validação

- Testar helpers de margem, clamp e escolha de dimensão com valores de monitor
  pequenos, normais e múltiplos.
- Em Wayland/Niri, testar monitor explícito por `KSHELL_OUTPUT`, abertura após
  reexposição da bateria, reabertura repetida e posicionamento no limite da
  tela.

## Decisão 3 — Coordinator e coexistência com Calendar

### Problema

`PopoverCoordinator` conhece somente `gtk::Popover` e um único estado ativo. A
guarda de identidade já evita que o fechamento atrasado de Calendar ou Volume
limpe o popup novo, mas ela não conhece a visibilidade nem o keyboard mode de
uma `ApplicationWindow` independente.

### Evidência no KShell

- `apps/kbar/src/ui/popover.rs:22-35` implementa a máquina de estado
  determinística de um popup ativo.
- `apps/kbar/src/ui/popover.rs:63-81` fecha o popup anterior e sempre altera a
  janela principal.
- `apps/kbar/src/ui/calendar.rs:140-164` possui Escape e focus explícitos,
  enquanto Volume não possui equivalente.

### Referência

Projeto: VibePanel

Arquivo: [`quick_settings/window.rs`](https://github.com/prankstr/vibepanel/blob/92063a36833ca8eda61a02485f8bfadcc73e1d43/crates/vibepanel/src/widgets/quick_settings/window.rs)

Conceito: um tracker de popup pode acompanhar uma window persistente e seu
estado lógico sem confundir fechamento de uma surface com a de outra.

### Decisão

Adaptar o coordinator existente, sem criar um framework global, para representar
o Volume como um owner de surface com operações de `show`/`hide` e Calendar como
o popup GTK atual. O estado deve continuar tendo um único popup lógico ativo,
mas a ação de teclado será por owner:

- Volume ativo: keyboard mode da surface de volume, janela principal em `None`;
- Calendar ativo: comportamento atual do Calendar, preservando sua validação
  manual e sem deixar Volume ativo;
- nenhum popup ativo: todas as superfícies retornam ao estado sem teclado
  temporário.

Fechamentos devem continuar condicionados à identidade do owner ativo. Abrir um
popup deve solicitar o fechamento do outro e só depois marcar o novo owner;
callbacks atrasados não podem limpar o estado mais recente.

### Alternativas rejeitadas

- Fazer o coordinator chamar sempre `window.set_keyboard_mode(OnDemand)`: é o
  acoplamento atual que a surface independente deve remover para Volume.
- Migrar Calendar para layer-shell nesta feature: amplia escopo sem evidência
  de que Calendar tenha o mesmo problema.
- Remover o coordinator e deixar cada popup fechar o outro diretamente:
  elimina a proteção de identidade já testada.

### Riscos

- A transição Calendar → Volume pode deixar a janela principal em `OnDemand`
  se o callback de Calendar for processado fora de ordem.
- Um click catcher que emita fechamento depois de abrir Calendar pode fechar a
  surface errada.

### Validação

- Estender os testes determinísticos de `PopoverState` para owners com
  lifecycle independente.
- Manualmente alternar Calendar/Volume rapidamente, usar Escape em cada um e
  confirmar que somente o owner ativo fecha.

## Decisão 4 — Autoridade do slider e sincronização assíncrona

### Problema

O KShell já tem uma máquina de interação rica, mas há uma diferença entre o
valor pendente da UI, o último `Set` aguardando o throttle e o valor confirmado
por uma leitura posterior. O relatório local identificou como possível uma
sequência em que uma resposta de `Sync` antiga chega depois de uma nova alteração
fora do pointer; não há evidência de ocorrência em todos os ambientes, mas o
fluxo é plausível e merece uma barreira explícita.

### Evidência no KShell

- `apps/kbar/src/ui/volume.rs:31-79` mantém `pointer_active`, `pending_value`,
  `waiting_for_sync` e tokens monotônicos.
- `apps/kbar/src/ui/volume.rs:186-234` combina `GestureClick` e `GestureDrag`.
- `apps/kbar/src/ui/volume.rs:438-475` usa throttle de 40 ms e flush final.
- `apps/kbar/src/ui/volume.rs:483-494` dá prioridade ao valor local durante
  pointer ou confirmação pendente.
- `apps/kbar/src/services/audio.rs:184-212` faz coalescing somente de `Set`
  contíguos; `:224-288` aplica valor otimista, espera 32 ms e pode tentar de
  novo após 16 ms.
- Os testes em `volume.rs:500-578` já protegem pointer lifecycle, token antigo
  e retorno da autoridade ao backend.

### Referências

Projeto: VibePanel

Arquivo: [`audio_card.rs`](https://github.com/prankstr/vibepanel/blob/92063a36833ca8eda61a02485f8bfadcc73e1d43/crates/vibepanel/src/widgets/quick_settings/audio_card.rs)

Conceitos: `AudioCardState`, `on_audio_changed`, `updating`,
`sync_audio_sink_list` e atualização de rows sem rebuild.

Projeto: DankMaterialShell

Arquivo: [`AudioSliderRow.qml`](https://github.com/AvengeMedia/DankMaterialShell/blob/master/quickshell/Modules/ControlCenter/Widgets/AudioSliderRow.qml)

Conceito: a binding backend → slider só é aplicada quando
`!volumeSlider.isDragging`; durante drag a UI local é autoridade.

Projeto: Noctalia

Arquivo: [`audio_tab.cpp`](https://github.com/noctalia-dev/noctalia/blob/main/src/shell/control_center/tabs/audio_tab.cpp)

Conceitos: `m_pending*`, `m_lastSent*`, timestamps, `m_syncing`,
`kVolumeSyncEpsilon` e a distinção entre valor desejado, enviado e confirmado.

### Decisão

Preservar `SliderInteractionState`, o throttle de 40 ms, o coalescing do
worker, o flush final e a confirmação por token. A estabilização deve tornar
explícitas as três noções, sem exigir três camadas de backend:

1. valor desejado local: último valor escolhido pela interação;
2. valor enviado: último `Set` que o worker retirou do fluxo de ações e tentou
   executar no `AudioBackend`; estar enviado não significa estar confirmado;
3. valor confirmado: última leitura válida que pertence à geração/Sync atual
   ou que chegou quando não há intenção pendente.

O protocolo de estado será:

1. A primeira mudança originada pelo usuário depois de não haver intenção
   pendente cria uma nova geração e grava o valor desejado.
2. Cada `Set` enviado pelo worker atualiza a noção de valor enviado, enquanto a
   UI pode continuar exibindo o desejado. O snapshot otimista produzido por
   `Set` não encerra a geração.
3. Ao finalizar click/drag, o flush final é seguido por `Sync` com token ligado
   à geração atual e ao valor solicitado. Se uma nova intenção surgir antes da
   confirmação, a geração/token anterior é invalidada e a nova interação deve
   produzir sua própria confirmação final.
4. Um `AudioStatus` com token diferente do token aguardado é antigo e não pode
   limpar o desejado. Um snapshot sem token também não confirma a geração
   enquanto houver `Sync` pendente; ele ainda pode atualizar mute e outputs.
5. Somente o `AudioStatus` com o token esperado, ou uma leitura posterior
   quando não há intenção pendente, atualiza o valor confirmado e libera a
   autoridade ao backend. O token é preservado pelo worker/agregador até ser
   consumido pela UI, sem criar um segundo bridge.
6. O valor pendente deve ser associado ao ID do output padrão quando esse dado
   estiver disponível. Uma troca de output invalida a confirmação dirigida ao
   dispositivo anterior, em vez de aplicar o valor a um novo default.

Uma nova intenção nunca pode ser limpa por snapshot anterior. Quando não houver
interação ou confirmação pendente, o snapshot externo volta a ser autoridade.
Mute e outputs não compartilham a autoridade do volume: mute externo e mudança
de output devem continuar sendo aplicados imediatamente, mesmo enquanto o
percentual do slider permanece local.

O epsilon atual de um ponto percentual e os delays de 32/16 ms serão tratados
como limites do backend `wpctl`, não como uma política visual de frame. O
intervalo de 16 ms do Noctalia NÃO será adotado enquanto cada `Set` criar um
subprocesso.

### Alternativas rejeitadas

- Trocar a máquina por um único booleano `updating_from_backend`: não cobre
  drag, click direto, tokens, throttle, confirmação atrasada ou subprocessos.
- Remover `SliderInteractionState` porque VibePanel tem uma flag `updating`:
  ignora limitações específicas do KShell.
- Reduzir o throttle para 16 ms: multiplica processos `wpctl` sem evidência de
  benefício.
- Tratar qualquer snapshot recebido depois de `Set` como confirmação: um
  snapshot pode ser antigo, otimista ou de outro output.

### Riscos

- Uma barreira excessiva pode impedir uma alteração externa legítima de
  aparecer depois do drag.
- Uma geração propagada de forma incompleta pode deixar o slider preso no valor
  local.
- A confirmação real do `wpctl` não é um evento formal; o design deve manter
  timeout, retry e estado indisponível.

### Validação

- Adicionar testes unitários para desired/sent/confirmed, geração antiga,
  drag consecutivo, click direto, cancelamento e epsilon.
- Manter testes de coalescing e ordem em torno de mute/SetDefault/Sync.
- Manualmente executar drag lento/rápido, alterar volume externamente durante o
  drag, repetir a interação antes da leitura final e conferir convergência.

## Decisão 5 — Diff estrutural e atualização das saídas

### Problema

O KShell já evita rebuild quando `OutputMenuState` é completamente igual, mas
uma mudança apenas do default altera esse estado e hoje pode recriar todas as
rows. Volume e mute são dados do sink padrão e não fazem parte da lista; a
distinção pode ser refinada para que a marcação ativa seja atualizada sem
trabalho estrutural.

### Evidência no KShell

- `apps/kbar/src/services/audio.rs:24-29` define `OutputDevice` por ID, nome e
  `is_default`.
- `apps/kbar/src/services/audio.rs:306-326` reconcilia outputs e default.
- `apps/kbar/src/ui/volume.rs:359-403` compara `OutputMenuState` e chama
  `remove_all()` quando ele muda.
- `apps/kbar/src/services/audio.rs:339-435` descarta o volume textual da linha
  e usa ID numérico para seleção.

### Referências

Projeto: VibePanel

Arquivo: [`audio_card.rs`](https://github.com/prankstr/vibepanel/blob/92063a36833ca8eda61a02485f8bfadcc73e1d43/crates/vibepanel/src/widgets/quick_settings/audio_card.rs)

Conceitos: `sync_audio_sink_list` e `sync_app_volume_list` diferenciam
identidade estrutural de atualização de dados.

Projeto: Noctalia

Arquivo: [`audio_tab.cpp`](https://github.com/noctalia-dev/noctalia/blob/main/src/shell/control_center/tabs/audio_tab.cpp)

Conceitos: `deviceListKey`, `rebuildLists`, `syncProgramVolumeRows` e rows
indexadas por identidade.

### Decisão

Separar no modelo da UI:

- estrutural: ID, nome e ordem das saídas;
- dinâmico: ID da saída padrão/marcação ativa e dados de volume/mute fora da
  lista.

O diff estrutural reconstrói as rows somente quando a coleção ou sua identidade
mudar. Uma mudança do default atualiza as marcações das rows existentes. Um
snapshot idêntico não faz trabalho. Não será criado mixer por aplicação nem uma
cache global de widgets.

### Alternativas rejeitadas

- Rebuild incondicional a cada `AudioStatus`: desnecessário e pode causar
  flicker/perda de foco.
- Comparar somente o nome: nomes podem ser normalizados ou repetidos; o ID
  existente é a identidade operacional.
- Introduzir `Revealer` e outputs recolhíveis agora: é redesign e não há
  requisito de altura/overflow confirmado.

### Riscos

- Reutilizar uma row com callback para o ID antigo pode enviar a saída errada.
- Atualizar somente a marcação sem considerar remoção/ordem pode deixar uma row
  fantasma.

### Validação

- Testar snapshot idêntico, volume/mute isolado, default alterado, nome alterado,
  ID removido/adicionado, ordem alterada e lista vazia.
- Manualmente observar a lista enquanto o polling atualiza volume, trocar
  output e reabrir o popup.

## Decisão 6 — Geometria e sistema de tema

### Problema

O popup atual usa classes e dimensões geradas para `GtkPopover`. Uma
`ApplicationWindow` independente pode precisar de seletores de conteúdo
equivalentes; editar `apps/kbar/src/ui/style.css` manualmente criaria uma
segunda fonte visual.

### Evidência no KShell

- `apps/kbar/src/ui/style.css` declara que é gerado.
- `crates/theme/src/tokens.rs:70-83` é a fonte de dimensões canônicas.
- `crates/theme/templates/kbar.css` é o template do CSS da Kbar.
- `docs/architecture/design-system.md` e ADR-0002 exigem tokens/templates como
  fonte única.

### Decisão

Manter `VOLUME_MODULE_WIDTH`, `VOLUME_POPOVER_WIDTH`, espaçamentos, radii,
focus/hover e tipografia atuais. Se a nova surface exigir uma classe de
conteúdo ou backdrop, ela será adicionada a `crates/theme/templates/kbar.css`,
seguida de `cargo run -p kshell-theme-gen -- --write` e `--check`. Nenhum
mockup, KDL ou token será alterado sem necessidade observável.

### Alternativas rejeitadas

- Editar CSS gerado diretamente: viola a fonte de verdade do tema.
- Aproveitar a migração para redesignar o popup: mistura estabilidade de
  lifecycle com mudança visual não solicitada.
- Criar tokens locais em `apps/kbar`: duplica geometria canônica.

### Riscos

O risco principal é um seletor que só funcione dentro do `GtkPopover`. A
superfície independente pode perder focus, hover ou estilos das output rows.

### Validação

A validação deve comparar screenshots/inspeção manual do módulo, popup, focus,
hover, output rows e `--check` do generator.

## Decisão 7 — Backend de áudio permanece `WpctlBackend`

### Problema

As referências VibePanel e Noctalia obtêm eventos por mainloop persistente,
PipeWire/WirePlumber ou `libpulse-binding`. O KShell atual usa `wpctl` por
subprocessos curtos, polling e limites explícitos; substituir o caminho agora
misturaria lifecycle de serviço com a estabilidade do popup.

### Evidência no KShell

- `apps/kbar/src/services/audio.rs:47-109` já possui `AudioBackend` e
  `WpctlBackend`.
- `apps/kbar/src/services/audio.rs:11-14` define polling de 500 ms, outputs de
  4 s e delays de Sync.
- `apps/kbar/src/services/command.rs:9-67` impõe timeout de 500 ms, execução sem
  shell e drenagem limitada.
- `apps/kbar/src/services/mod.rs:32-40,77-124` mantém workers e deduplicação de
  snapshots.

### Referências

Projeto: VibePanel

Arquivo: [`audio.rs`](https://github.com/prankstr/vibepanel/blob/92063a36833ca8eda61a02485f8bfadcc73e1d43/crates/vibepanel/src/services/audio.rs)

Conceito: `libpulse-binding`, `ThreadedMainloop`, subscriptions e comunicação
worker → GTK persistente.

Projeto: Noctalia

Arquivos: [`pipewire_service.h`](https://github.com/noctalia-dev/noctalia/blob/main/src/pipewire/pipewire_service.h), [`wireplumber_mixer.cpp`](https://github.com/noctalia-dev/noctalia/blob/main/src/pipewire/wireplumber_mixer.cpp)

Conceito: `WpCore`/`GMainContext`, callbacks de mudança, valores pendentes por
device e proteção contra readback atrasado.

### Decisão

Preservar a trait, o worker, o polling, os limites de comando, o coalescing e o
fallback `WpctlBackend`. A feature pode ajustar o contrato de sincronização e
os testes, mas não adicionará biblioteca de áudio, thread persistente,
subscription PipeWire ou protocolo novo.

### Alternativas rejeitadas

- Portar `libpulse-binding` do VibePanel agora: mudança de dependência e
  lifecycle fora do popup.
- Adotar PipeWire/WirePlumber do Noctalia agora: exige integração persistente e
  modelo de eventos que não existe no KShell.
- Aumentar polling para compensar o subprocesso: aumenta custo e não fornece
  confirmação confiável.

### Riscos

- Manter polling pode deixar a confirmação externa atrasada em comparação com
  um backend orientado a eventos.
- Alterar o contrato de sincronização sem preservar os limites de comando pode
  fazer a UI parecer responsiva enquanto o worker fica bloqueado.

### Validação

Os testes de parsing, timeout, coalescing e estado devem continuar passando.
Qualquer alteração em serviços exige `cargo check --workspace`, testes do
workspace e verificação manual com `wpctl` disponível e indisponível.

## Decisão 8 — UX futura: outputs, OSD e mixer

### Problema

VibePanel e os outros shells apresentam recursos como `Revealer`, OSD, streams
por aplicação, microfone e proteção de feedback sonoro. Eles são referências de
possibilidades, não evidência de que o popup compacto do KShell precise desses
recursos agora.

### Evidência no KShell

- `apps/kbar/src/ui/volume.rs:280-293` já transforma scroll no módulo em uma
  ação de ajuste de cinco pontos, mas não há uma surface separada de feedback
  rápido.
- `apps/kbar/src/ui/volume.rs:134-255` mantém o popup como painel de
  configuração com slider, mute e saídas; não há mixer por aplicação ou
  streams no modelo atual.
- `apps/kbar/src/services/audio.rs:16-29` modela somente o sink padrão e a
  lista de outputs, sem estado por programa.

### Referência

Projeto: VibePanel

Arquivo: [`osd.rs`](https://github.com/prankstr/vibepanel/blob/92063a36833ca8eda61a02485f8bfadcc73e1d43/crates/vibepanel/src/widgets/osd.rs)

Conceitos: widget/surface de OSD separado do Quick Settings, lifecycle curto e
feedback rápido sem transformar o painel de configuração em um HUD.

### Alternativas rejeitadas

- Adicionar OSD junto com a nova surface: mistura duas frequências de lifecycle
  e cria requisitos de auto-hide antes de o popup estar estável.
- Tornar outputs recolhíveis agora: não há evidência de overflow no popup atual
  e a mudança alteraria o contrato visual.
- Adicionar mixer por aplicação por imitação de Noctalia: exigiria identidade,
  estado e backend por stream ausentes no KShell.

### Decisão

- Outputs recolhíveis: **ADIAR**; a lista direta atual cabe no contrato e não há
  evidência de overflow que justifique redesign.
- OSD separado: **ADIAR** para `007-volume-osd`; o popup continua sendo painel
  de configuração e o OSD futuro não deve receber teclado.
- Mixer por aplicação: **NÃO APLICAR** em 005; permanece fora da feature e
  exigiria backend persistente/estado por stream.
- Microfone e Bluetooth: fora da feature e sem task em 005.

### Riscos

- Adiar OSD mantém o scroll sem feedback visual dedicado, mas preserva o escopo
  e evita uma segunda surface com auto-hide nesta feature.
- Adiar mixer mantém o modelo de sink padrão simples, mas não resolve controle
  por aplicação; isso é uma limitação explícita, não uma promessa de 005.

### Validação

- Confirmar manualmente que o popup completo continua sendo a única surface de
  configuração e que scroll/mute permanecem funcionando.
- Registrar qualquer necessidade real de OSD, overflow ou streams como
  requisito de `007-volume-osd` ou `006-audio-service`, sem ampliar 005.

## Resultado das hipóteses

| Hipótese | Classificação | Síntese da decisão |
| --- | --- | --- |
| 1. Surface independente | APLICAR AGORA | `ApplicationWindow` layer-shell própria para Volume. |
| 2. Surface viva com show/hide | APLICAR AGORA | Criação lazy, reuse e lifecycle lógico persistente. |
| 3. Keyboard mode na surface própria | APLICAR AGORA | Volume usa seu próprio mode enquanto ativo. |
| 4. Janela principal sem keyboard mode do Volume | APLICAR AGORA | Kbar permanece `None` para Volume; Calendar mantém seu contrato atual. |
| 5. Posicionamento por módulo/monitor/bar | APLICAR AGORA | Margens calculadas com monitor, alocação e tokens atuais. |
| 6. Lifecycle/foco não interferir na Kbar | APLICAR AGORA | Sem parent GTK, sem exclusive zone e com teste de geometria. |
| 7. Prioridade local durante interação | APLICAR AGORA | Preservar e reforçar a máquina de interação. |
| 8. Backend autoridade fora da interação | APLICAR AGORA | Snapshot válido reassume depois da confirmação. |
| 9. Snapshot antigo sem flicker | APLICAR AGORA | Geração/token e testes; não aceitar confirmação indiscriminada. |
| 10. Preservar `SliderInteractionState` | APLICAR AGORA | Não substituir por um booleano sem prova. |
| 11. Preservar `AudioBackend` | APLICAR AGORA | Limite de serviço permanece. |
| 12. Manter `WpctlBackend` | APLICAR AGORA | Nenhuma migração de backend em 005. |
| 13. Backend persistente futuro | PREPARAR AGORA | Manter trait, limites e documentação de 006; a implementação do backend fica fora de 005. |
| 14. Não aumentar subprocessos | APLICAR AGORA | Preservar throttle/coalescing e não adotar 16 ms. |
| 15. Volume não reconstrói outputs | APLICAR AGORA | Separar dados de volume do diff estrutural. |
| 16. Estrutural versus dinâmico | APLICAR AGORA | Rebuild só para identidade; default/estado atualizam rows. |
| 17. Outputs recolhíveis | ADIAR | Sem requisito de overflow ou redesign. |
| 18. OSD separado | ADIAR | Avaliar como `007-volume-osd`. |
| 19. Mixer por aplicação fora | NÃO APLICAR | Fora de 005 e dependente de backend/estado por stream. |

## Sequência de implementação

1. **T001 — Baseline e proteção de comportamento existente:** testes
   determinísticos antes da mudança estrutural.
2. **T002 — Surface independente para o volume:** infraestrutura mínima de
   `ApplicationWindow`, layer-shell, monitor, namespace e owner.
3. **T003 — Lifecycle e posicionamento:** criação lazy, show/hide, click
   catcher, Escape, dimensões e margens.
4. **T004 — Migrar conteúdo atual do volume:** conteúdo, ações e tema sem
   redesign.
5. **T005 — Estabilizar sincronização do slider:** autoridade local, confirmação
   e proteção de snapshots antigos mantendo `wpctl`.
6. **T006 — Atualização incremental dos outputs:** diff estrutural e marcação
   dinâmica sem rebuild desnecessário.
7. **T007 — Integração entre popups:** coordinator, Calendar, foco e fechamentos
   atrasados.
8. **T008 — Validação integrada:** gates do workspace e matriz manual Wayland/Niri.

## Feature futura 006 — `audio-service`

Deve ser uma feature separada se houver decisão de introduzir um backend
persistent/event-driven. O escopo provável é:

- implementar `PersistentAudioBackend` atrás de `AudioBackend`;
- manter `WpctlBackend` como fallback inicial;
- mainloop/thread/subscription e lifecycle explícitos;
- eventos de sink/source/default e metadados de output;
- confirmação e proteção contra readback atrasado no serviço;
- menor dependência de polling e subprocessos;
- eventual suporte a streams por aplicação somente depois de um modelo de
  segurança e identidade adequado.

Nenhuma dessas mudanças é task de 005.

## Feature futura 007 — `volume-osd`

Deve separar o feedback rápido do painel completo:

- scroll/hotkey altera volume;
- uma surface pequena mostra o valor por tempo limitado;
- auto-hide;
- OSD sem keyboard mode próprio;
- popup de volume continua sendo a superfície de configuração.

O padrão conceitual pode ser estudado no VibePanel, mas não há implementação
nem task correspondente nesta feature.
