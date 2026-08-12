# Tasks da funcionalidade 005: popup de volume estável da Kbar

As tasks abaixo são unidades funcionais verificáveis. Elas não autorizam
mixer por aplicação, backend persistente, OSD ou redesign fora do escopo de
spec.md e do delta descrito em plan.md.

## T001 — Baseline e proteção de comportamento existente

ID: T001
Título: Baseline e proteção de comportamento existente

### Objetivo

Registrar em testes determinísticos os contratos que a mudança de surface não
pode quebrar: autoridade local do slider, sincronização, coalescing/ordem de
ações, diff de outputs e exclusividade lógica dos popups.

### Contexto

O KShell já possui testes junto da implementação em
apps/kbar/src/ui/volume.rs, apps/kbar/src/ui/popover.rs e
apps/kbar/src/services/audio.rs. Eles cobrem parte da máquina de interação,
tokens e ações, mas não devem ser substituídos por testes de GTK real. Esta
task é a barreira antes de mover o popup para outra surface.

### Arquivos prováveis

- apps/kbar/src/ui/volume.rs
- apps/kbar/src/ui/popover.rs
- apps/kbar/src/services/audio.rs
- novos módulos de estado determinístico em apps/kbar/src/ somente se uma
  extração pequena reduzir acoplamento ao GTK

### Referências

- Código local: SliderInteractionState e seus testes em
  apps/kbar/src/ui/volume.rs:31-79,500-578.
- Código local: PopoverState e testes de fechamento atrasado em
  apps/kbar/src/ui/popover.rs:17-41,84-108.
- Código local: coalescing e ordem de VolumeAction em
  apps/kbar/src/services/audio.rs:175-212,597-668.
- Constituição: princípio de testar estado e parsing fora do GTK em
  .specify/memory/constitution.md.

### Implementação esperada

- Cobrir que valor local vence snapshot externo durante pointer e confirmação
  pendente.
- Cobrir que complete_sync aceita somente o token esperado e que um token
  antigo não finaliza interação posterior.
- Cobrir drag finalizado, cancelado, clique direto e nova intenção enquanto
  uma confirmação anterior está pendente.
- Cobrir que Set contíguos são coalescidos sem atravessar ToggleMute,
  SetDefault ou Sync.
- Cobrir snapshot de outputs idêntico, mudança de identidade e mudança do
  default, definindo a diferença entre estrutural e dinâmica antes da migração.
- Preservar os testes existentes; adicionar helpers puros somente quando forem
  necessários para expressar a decisão de autoridade.

### Não fazer

- Não criar ApplicationWindow, click catcher ou teste de renderização GTK.
- Não alterar throttle, delays, backend, comandos wpctl ou tokens visuais.
- Não transformar hipóteses de geometry em testes unitários que não possam
  observar Wayland.

### Critérios de aceite

- A suíte determinística prova prioridade local, confirmação correta e
  invalidação de token/geração antiga.
- A suíte prova coalescing e ordem de ações existentes.
- A suíte distingue snapshot de outputs idêntico de mudança real.
- Todos os testes anteriores continuam passando sem mudança de comportamento
  visível.

### Validação

~~~sh
cargo test -p kbar
cargo test --workspace
~~~

Inspecionar o diff para confirmar que somente lógica determinística foi coberta.

### Dependências

Nenhuma. Esta task deve ser concluída antes de T002.

### Riscos

Um teste excessivamente acoplado ao layout atual pode dificultar a migração.
Preferir entradas, estados e decisões observáveis a snapshots de widgets.

### Notas de implementação

pending_value e pending_set_value não são a mesma coisa: o primeiro é a
intenção visual local e o segundo é o valor aguardando envio. A baseline deve
manter essa distinção explícita.

## T002 — Surface independente para o volume

ID: T002
Título: Surface independente para o volume

### Objetivo

Criar a infraestrutura mínima para o volume existir em uma
ApplicationWindow layer-shell própria, sem ainda migrar toda a lógica do
conteúdo.

### Contexto

Hoje VolumeWidget::new cria um gtk::Popover, parenta-o em volume_item e o
registra no coordinator. A nova surface precisa continuar pertencendo à mesma
aplicação GTK, mas não à árvore da janela principal da barra.

### Arquivos prováveis

- apps/kbar/src/ui/volume.rs
- apps/kbar/src/ui/popover.rs
- apps/kbar/src/app.rs para reutilizar o monitor já resolvido pelo
  `OutputContext` privado em `:156-185`, se necessário
- crates/niri/src/lib.rs somente para adicionar o literal de namespace
  dedicado, caso a regra de identificadores de `ADR-0003` seja aplicada

### Referências

- VibePanel:
  https://github.com/prankstr/vibepanel/blob/92063a36833ca8eda61a02485f8bfadcc73e1d43/crates/vibepanel/src/widgets/layer_shell_popover.rs,
  especialmente LayerShellPopover::ensure_window_shell e
  ensure_click_catcher.
- KShell local: configuração da barra em apps/kbar/src/app.rs:69-98 e
  seleção de monitor em apps/kbar/src/app.rs:156-185.
- KShell local: identificadores compartilhados em crates/niri/src/lib.rs.

### Implementação esperada

- Criar uma gtk::ApplicationWindow associada à mesma gtk::Application.
- Configurar layer Top, exclusive zone zero, superfície sem decoração e
  namespace dedicado, sem reutilizar o namespace da barra.
- Aplicar o mesmo monitor que `OutputContext::resolve` já seleciona em
  `apps/kbar/src/app.rs:156-185`; não criar um novo tipo de contexto nem mover
  essa resolução para a surface.
- Manter KeyboardMode::None na janela principal quando Volume for o único
  popup; reservar OnDemand para a surface de Volume enquanto ativa.
- Definir ownership claro: o handle da surface deve ser mantido por Volume ou
  pelo coordinator e ser reutilizável em chamadas posteriores.
- Criar a infraestrutura mínima de click catcher reutilizável se ela for
  necessária para click-outside; não iniciar uma abstração global para outros
  popups.
- Expor operações de mostrar/ocultar suficientes para T003, ainda que o
  conteúdo migrado permaneça temporariamente no lugar atual durante a transição.
  Durante esta task a nova surface deve permanecer oculta e o `GtkPopover`
  antigo deve continuar sendo o único caminho visível; T003/T004 farão a troca
  explícita.

### Não fazer

- Não migrar Calendar nesta task.
- Não portar SurfaceHeightFreeze, blur, animações ou Revealer do VibePanel.
- Não trocar AudioBackend, WpctlBackend, polling ou comandos.
- Não introduzir um novo processo, toolkit ou framework genérico de surfaces.

### Critérios de aceite

- Existe uma surface Volume independente da árvore da ApplicationWindow
  principal.
- A surface usa layer-shell, monitor correto, namespace dedicado e não reserva
  exclusive zone.
- A janela principal não recebe KeyboardMode::OnDemand apenas por Volume.
- A criação do owner é única e não cria surface duplicada em chamadas repetidas.
- Durante a transição, não há dois popups visíveis nem dois gatilhos de abertura
  ativos.
- Calendar e o conteúdo existente ainda compilam; qualquer comportamento
  temporariamente incompleto fica limitado à transição para T003/T004.

### Validação

~~~sh
cargo fmt --all -- --check
cargo check -p kbar
cargo test -p kbar
~~~

Em Wayland/Niri, confirmar que a surface pode ser criada, recebe o monitor
selecionado e não altera a exclusive zone da barra.

### Dependências

T001.

### Riscos

Uma surface layer-shell sem anchors/margens válidos pode não aparecer. Uma
surface de click catcher mal ordenada pode interceptar o slider; testar a ordem
antes de conectar todos os callbacks.

### Notas de implementação

Se o namespace for adicionado a `crates/niri`, ele deve ser somente um
identificador dedicado, como `my-shell-volume-popup`, alinhado ao ADR-0003; não
criar tipo, serviço ou fragmento KDL enquanto não houver uma regra Niri que o
consuma.

## T003 — Lifecycle e posicionamento da surface de volume

ID: T003
Título: Lifecycle e posicionamento da surface de volume

### Objetivo

Fazer a surface abrir, fechar, reposicionar e reabrir de forma persistente,
com click-outside e Escape, sem interferir na geometria ou no keyboard mode da
barra principal.

### Contexto

Uma surface independente não pode usar set_parent nem depender do
posicionamento automático do GtkPopover. O popup atual tem largura de 240 px,
offset derivado de BAR_HEIGHT/STATUS_ICON_SIZE e altura variável conforme a
lista de saídas.

### Arquivos prováveis

- apps/kbar/src/ui/volume.rs
- apps/kbar/src/ui/popover.rs
- apps/kbar/src/app.rs ou helper local de geometry
- crates/theme/templates/kbar.css somente se o novo root exigir seletor

### Referências

- VibePanel:
  https://github.com/prankstr/vibepanel/blob/92063a36833ca8eda61a02485f8bfadcc73e1d43/crates/vibepanel/src/widgets/layer_shell_popover.rs,
  show_at, hide, update_position_for_size, setup_esc_handler.
- VibePanel:
  https://github.com/prankstr/vibepanel/blob/92063a36833ca8eda61a02485f8bfadcc73e1d43/crates/vibepanel/src/widgets/quick_settings/window.rs,
  toggle_at, show_panel, hide_panel, cached_width e cached_height.
- KShell local: tokens de geometria em crates/theme/src/tokens.rs:70-83.

### Implementação esperada

- Criar a surface somente na primeira abertura e reutilizá-la após hide.
- Manter um estado lógico aberto/fechado separado de set_visible.
- Registrar uma única vez os handlers de show/hide, Escape, click-outside e
  abertura/fechamento; não criar subscriptions de áudio por abertura. O bridge
  global de `StatusUpdate::Audio` continua sendo a única entrada de estado.
- Posicionar a surface no monitor da barra com base na alocação do módulo,
  largura/última dimensão válida do popup, margem da barra e offset tokenizado.
- Usar cache de dimensões quando a surface oculta fornecer zero; limitar o
  resultado ao monitor em resoluções pequenas.
- Implementar Escape na própria surface e click-outside no click catcher.
- Ao abrir ou reabrir, dar foco ao botão de mute, que é o primeiro controle
  interativo do conteúdo atual; ao fechar, remover o foco da surface e
  desativar seu keyboard mode.
- Usar o offset vertical observável atual, derivado de
  `(BAR_HEIGHT - STATUS_ICON_SIZE) / 2 + SPACE_2`, convertido para a margem da
  surface layer-shell; calcular a margem horizontal a partir do retângulo do
  módulo e limitar o popup dentro do monitor.
- Garantir que fechar por qualquer caminho desative a surface de Volume e seu
  keyboard mode, sem resetar Calendar que já tenha se tornado o owner ativo.
- Reabrir deve atualizar outputs e preservar o conteúdo vivo, sem criar outra
  surface.

### Não fazer

- Não alterar o tamanho reservado do módulo, a exclusive zone ou o layout do
  StatusWidget.
- Não redesenhar o popup, adicionar outputs recolhíveis ou migrar Calendar.
- Não resolver a posição por animação contínua ou polling de frame.
- Não assumir que o GtkPopover causava reflow; validar a geometria observada.

### Critérios de aceite

- Abrir, fechar, reabrir e alternar rapidamente não cria surfaces nem callbacks
  duplicados.
- Escape e click-outside fecham Volume; clique dentro não fecha o popup.
- O popup aparece no monitor e no lado correto do módulo, inclusive após
  reabertura e com dimensões inicialmente desconhecidas; fica abaixo do módulo
  com o offset vertical tokenizado e é limitado aos limites do monitor.
- A janela principal permanece sem keyboard mode temporário por causa de
  Volume.
- Ao abrir/reabrir, o botão de mute recebe foco; ao fechar, o foco é liberado e
  nenhum handler de teclado da surface continua ativo.
- A largura, a altura e a posição alocada do módulo de volume não mudam ao
  abrir/fechar ou ao atualizar o conteúdo.

### Validação

~~~sh
cargo fmt --all -- --check
cargo test -p kbar
cargo check -p kbar
~~~

Manual Wayland/Niri: abrir/fechar repetidamente, Escape, click-outside,
monitor explícito, bordas do monitor, foco e inspeção do módulo antes/depois.

### Dependências

T002.

### Riscos

O retângulo do módulo pode estar indisponível durante a primeira apresentação.
O click catcher pode roubar eventos do popup se a ordem de surfaces não for
validada.

### Notas de implementação

Para a barra atual, tratar o topo como posição suportada. Não criar uma matriz
de posições laterais sem uma mudança explícita na arquitetura da Kbar.

## T004 — Migrar o conteúdo atual do volume

ID: T004
Título: Migrar o conteúdo atual do volume

### Objetivo

Mover o conteúdo existente para a nova surface sem redesign e sem introduzir
novas capacidades de áudio.

### Contexto

O popup atual contém cabeçalho com ícone/mute/percentual, GtkScale, seção de
saídas, estado vazio e callbacks para ToggleMute, Set e SetDefault. Essas
interações precisam sobreviver à troca de owner e de parent.

### Arquivos prováveis

- apps/kbar/src/ui/volume.rs
- apps/kbar/src/ui/status.rs somente para ajustar o handle do volume
- apps/kbar/src/ui/popover.rs
- crates/theme/templates/kbar.css e o output gerado
  apps/kbar/src/ui/style.css, somente se necessário

### Referências

- KShell local: construção e callbacks em apps/kbar/src/ui/volume.rs:134-306.
- KShell local: aplicação de status em apps/kbar/src/ui/volume.rs:310-404.
- KShell local: tokens VOLUME_MODULE_WIDTH, VOLUME_POPOVER_WIDTH e
  VOLUME_OUTPUT_ROW_HEIGHT em crates/theme/src/tokens.rs:79-83.
- Arquitetura visual: docs/architecture/design-system.md e ADR-0002.

### Implementação esperada

- Transferir header, mute button, percentuais, slider, output section, estado
  vazio e callbacks para o conteúdo da ApplicationWindow de Volume.
- Remover o `GtkPopover` antigo, seu `set_parent(&volume_item)` e seu registro
  no coordinator ao concluir a migração; o módulo deve ter um único caminho de
  abertura para a nova surface.
- Manter o módulo da barra como trigger visual e preservar ícone, label,
  tooltip, focus/hover e a largura de 60 px.
- Manter RefreshOutputs ao abrir, ToggleMute no botão/clique do meio,
  Adjust(5/-5), Set, Sync e SetDefault com a ordem atual.
- Manter o slider em 0..=100, incremento de 1/5 e click direto.
- Preservar o estado indisponível quando o backend não fornece percentual.
- Reusar tokens e classes existentes; se a nova hierarquia exigir CSS,
  alterar o template e regenerar os outputs, nunca editar CSS gerado como fonte.

### Não fazer

- Não adicionar mixer por aplicação, microfone, Bluetooth, OSD ou outputs
  recolhíveis.
- Não alterar AudioBackend, WpctlBackend, polling, timeout ou throttle.
- Não remover SliderInteractionState nem simplificar handlers nesta task.
- Não alterar a paleta, tipografia ou geometria aprovada sem necessidade de
  compatibilidade com a nova surface.

### Critérios de aceite

- Todas as funções atuais de volume continuam disponíveis e acionam as mesmas
  VolumeAction.
- O popup mostra percentual, mute, slider, lista, default e estado vazio como
  antes.
- O módulo de volume mantém largura, icon size, label reservado e classes de
  tema.
- Reabrir a surface mostra o estado mais recente sem duplicar callbacks.
- Se CSS/template foi alterado, o output gerado corresponde ao template.

### Validação

~~~sh
cargo fmt --all -- --check
cargo test -p kbar
cargo run -p kshell-theme-gen -- --check
cargo build -p kbar
~~~

Manual Wayland/Niri: abrir popup, conferir mute, percentual, slider, outputs,
default, estado vazio, focus/hover e click direto.

### Dependências

T001, T002 e T003.

### Riscos

Ao retirar set_parent, seletores GTK que dependiam de > contents podem não ser
aplicados. O conteúdo pode ficar visualmente correto, mas não interativo, se o
click catcher ou a superfície não encaminhar input corretamente.

### Notas de implementação

O conteúdo deve continuar sendo atualizado pelo bridge existente
StatusUpdate::Audio; não criar uma segunda leitura direta de áudio na UI.

## T005 — Estabilizar sincronização do slider

ID: T005
Título: Estabilizar sincronização do slider

### Objetivo

Garantir que o valor desejado local prevaleça durante interação, que snapshots
antigos não causem flicker e que o backend reassuma autoridade após a
confirmação final, mantendo o backend por wpctl e as proteções atuais.

### Contexto

O fluxo atual é input GTK → estado local → throttle de 40 ms → VolumeAction
→ WpctlBackend → snapshot otimista/Sync → bridge GTK. A UI já usa
pointer_active, pending_value, waiting_for_sync e token; o worker já faz
coalescing e leitura após 32 ms com retry de 16 ms.

### Arquivos prováveis

- apps/kbar/src/ui/volume.rs
- apps/kbar/src/services/audio.rs
- apps/kbar/src/services/mod.rs somente se o metadata de confirmação precisar
  atravessar o agregador

### Referências

- VibePanel:
  https://github.com/prankstr/vibepanel/blob/92063a36833ca8eda61a02485f8bfadcc73e1d43/crates/vibepanel/src/widgets/quick_settings/audio_card.rs,
  on_audio_changed, flag updating e sincronização incremental.
- DankMaterialShell:
  https://github.com/AvengeMedia/DankMaterialShell/blob/master/quickshell/Modules/ControlCenter/Widgets/AudioSliderRow.qml,
  binding backend → slider condicionada a !isDragging.
- Noctalia:
  https://github.com/noctalia-dev/noctalia/blob/main/src/shell/control_center/tabs/audio_tab.cpp,
  m_pending*, m_lastSent*, timestamps, m_syncing e epsilon.
- KShell local: SliderInteractionState em
  apps/kbar/src/ui/volume.rs:31-79 e Sync em
  apps/kbar/src/services/audio.rs:224-288.

### Implementação esperada

- Preservar GestureClick para click direto/limites de press/release e
  GestureDrag para o lifecycle de drag/cancelamento.
- Manter a regra: durante pointer ou confirmação pendente, o valor local
  governa o thumb, labels e ícone; sem interação pendente, o backend governa.
- Tornar explícita, em estado testável, a diferença entre valor desejado local,
  último valor enviado e confirmação do backend. Não é obrigatório expor três
  campos públicos, mas a decisão deve ser observável nos testes.
- Associar a confirmação final à interação/geração atual. Uma nova intenção
  deve invalidar a confirmação anterior e não pode ser concluída por token ou
  snapshot antigo.
- Usar esta tabela de transições como contrato de implementação:

  | Estado lógico | Evento | Volume exibido | Mute/outputs | Próximo estado |
  | --- | --- | --- | --- | --- |
  | Ocioso | snapshot sem intenção local | valor confirmado pelo backend | backend | Ocioso |
  | Intenção local | primeiro input do usuário | valor desejado local | backend continua aplicável | Envio pendente |
  | Envio pendente | worker executa `Set` | valor desejado local; atualiza valor enviado | backend pode atualizar mute/outputs | Aguardando `Sync` |
  | Aguardando `Sync` | snapshot sem token ou com token diferente | mantém desejado; não confirma | aplica mute/outputs | Aguardando `Sync` |
  | Aguardando `Sync` | `Sync` com token da geração atual | aceita leitura como confirmado, mesmo se divergir por mudança externa | aplica todos os campos | Ocioso |
  | Qualquer estado pendente | novo input do usuário | substitui desejado e cria nova geração | mantém mute/outputs do backend | Intenção local |

  Nesta tabela, enviado significa que o worker retirou `Set` da fila e tentou
  executá-lo, não apenas que a UI o colocou no channel. Um snapshot sem token
  somente é autoridade para volume no estado Ocioso.
- Manter flush do valor final, coalescing de Set, ordem em torno de mute e
  troca de output, leitura otimista limitada e retorno ao backend após Sync.
- Manter o epsilon efetivo atual de um ponto percentual salvo evidência local
  de que outro limite é necessário; documentar a escolha no teste.
- Manter throttle de 40 ms e delays de 32/16 ms enquanto cada Set for um
  subprocesso. Qualquer alteração deve ser motivada por medição e não pelo
  valor de 16 ms do Noctalia.
- Cobrir explicitamente drag lento, drag rápido, click direto, cancelamento,
  nova interação antes da confirmação, atualização externa durante drag,
  atualização externa após drag e backend indisponível.

### Não fazer

- Não substituir a máquina por bool updating_from_backend.
- Não remover tokens, pending_value, flush, coalescing ou retry sem prova
  equivalente nos testes.
- Não considerar todo snapshot após Set como confirmação.
- Não aumentar a frequência de wpctl, implementar libpulse-binding, PipeWire
  persistente ou mixer por aplicação.

### Critérios de aceite

- O thumb e percentuais não recuam diante de snapshot antigo durante interação.
- Click direto, drag lento, drag rápido e cancelamento não deixam estado preso.
- O último valor local é enviado mesmo quando há timer pendente.
- Token/geração antigo não finaliza uma nova interação.
- Uma confirmação correspondente libera o backend como autoridade.
- Alteração externa com popup aberto/fechado aparece depois que não há intenção
  local pendente.
- A quantidade e a frequência de Set continuam limitadas pelo throttle e
  coalescing existentes.
- Mute externo e mudanças de output continuam sendo aplicados mesmo quando o
  percentual permanece protegido pela geração local.

### Validação

~~~sh
cargo test -p kbar
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
~~~

Manual Wayland/Niri/PipeWire: drag lento/rápido, click direto, alteração por
wpctl durante drag, alteração depois do release, duas interações rápidas,
mute/SetDefault intercalados e backend indisponível.

### Dependências

T001 e T004.

### Riscos

Se a nova barreira nunca for liberada, o slider pode parecer correto durante o
drag mas ignorar uma alteração externa legítima depois. Se for liberada cedo,
um snapshot antigo pode produzir flicker. Os testes devem cobrir ambas as
transições.

### Notas de implementação

O modelo de Noctalia é uma referência de estado, não uma autorização para
copiar kVolumeCommitInterval = 16ms. O caminho KShell continua limitado pelo
custo de std::process::Command e pelos timeouts de services/command.rs.

## T006 — Atualização incremental dos outputs

ID: T006
Título: Atualização incremental dos outputs

### Objetivo

Evitar reconstrução estrutural quando somente dados dinâmicos mudarem e
preservar a identidade, seleção e callbacks corretos das saídas.

### Contexto

O KShell já compara OutputMenuState antes de chamar remove_all, mas o default
está misturado à igualdade estrutural. OutputDevice atualmente contém ID, nome
e is_default; o volume do sink não faz parte da linha.

### Arquivos prováveis

- apps/kbar/src/ui/volume.rs
- apps/kbar/src/services/audio.rs
- testes unitários nos mesmos arquivos

### Referências

- VibePanel:
  https://github.com/prankstr/vibepanel/blob/92063a36833ca8eda61a02485f8bfadcc73e1d43/crates/vibepanel/src/widgets/quick_settings/audio_card.rs,
  sync_audio_sink_list e sync_app_volume_list.
- Noctalia:
  https://github.com/noctalia-dev/noctalia/blob/main/src/shell/control_center/tabs/audio_tab.cpp,
  deviceListKey, rebuildLists e syncProgramVolumeRows.
- KShell local: diff atual em apps/kbar/src/ui/volume.rs:359-403 e
  parser/identidade em apps/kbar/src/services/audio.rs:368-435.

### Implementação esperada

- Definir identidade estrutural como coleção ordenada de ID, nome e ordem.
- Tratar ID da saída padrão/marcação ativa como estado dinâmico: atualizar as
  rows existentes quando apenas o default mudar.
- Manter volume e mute fora do diff de lista; mudanças neles não devem chamar
  remove_all nem reinstalar callbacks.
- Reconstruir rows somente quando ID, nome, ordem, inclusão ou remoção mudar.
- Garantir que cada callback continue capturando o ID correto após atualização
  ou rebuild.
- Preservar lista vazia, fallback de uma única saída sem marcador default,
  seleção por ID e atualização imediata após SetDefault.

### Não fazer

- Não criar mixer por aplicação, rows de streams, microfone ou Bluetooth.
- Não usar somente nome como identidade.
- Não adicionar Revealer, scroll container ou outputs recolhíveis.
- Não reconstruir a lista para cada AudioStatus por simplicidade.

### Critérios de aceite

- Snapshot idêntico não altera rows.
- Volume/mute isolado não altera rows.
- Default alterado atualiza somente marcações/estado ativo observável.
- ID/nome/ordem adicionados, removidos ou alterados atualizam a coleção.
- Selecionar uma saída continua enviando seu ID correto.
- A lista permanece correta após reabrir a surface e após polling de 4 s.

### Validação

~~~sh
cargo test -p kbar
cargo clippy --workspace --all-targets -- -D warnings
~~~

Manual: trocar volume, mute e default; conectar/desconectar uma saída quando
possível; observar ausência de flicker e seleção ativa correta.

### Dependências

T001 e T004.

### Riscos

Atualização incremental incorreta pode deixar a marcação de duas saídas ativa
ou manter uma row removida. Cobrir coleção vazia e transições rápidas.

### Notas de implementação

Não criar uma cache global de widgets. O cache deve ficar limitado ao conteúdo
do popup e ser descartado quando a identidade estrutural realmente mudar.

## T007 — Integração entre popups

ID: T007
Título: Integração entre popups

### Objetivo

Revisar a coordenação entre a surface independente de Volume e o Calendar,
preservando exclusividade, foco e proteção contra callbacks de fechamento fora
de ordem.

### Contexto

O coordinator atual guarda active: Option<PopoverId>, registra duas
gtk::Popover e altera o keyboard mode da janela principal. Após T002/T003,
Volume terá uma ApplicationWindow e click catcher próprios, enquanto Calendar
continuará sendo um GtkPopover.

### Arquivos prováveis

- apps/kbar/src/ui/popover.rs
- apps/kbar/src/ui/volume.rs
- apps/kbar/src/ui/calendar.rs somente para ajustar hooks existentes
- apps/kbar/src/app.rs somente se ownership/lifecycle exigir passagem de
  contexto

### Referências

- KShell local: PopoverState e guardas em
  apps/kbar/src/ui/popover.rs:17-81.
- KShell local: Escape/focus de Calendar em
  apps/kbar/src/ui/calendar.rs:140-164.
- VibePanel:
  https://github.com/prankstr/vibepanel/blob/92063a36833ca8eda61a02485f8bfadcc73e1d43/crates/vibepanel/src/widgets/quick_settings/window.rs,
  estado lógico e show_panel/hide_panel.

### Implementação esperada

- Manter um único owner lógico ativo entre Volume e Calendar.
- Abrir Volume deve fechar Calendar; abrir Calendar deve esconder Volume e seu
  click catcher.
- Um callback de fechamento atrasado deve ser aceito somente se ainda
  corresponder ao owner ativo.
- Volume deve controlar seu próprio keyboard mode; a janela principal não deve
  ser ativada por Volume.
- Calendar deve preservar Escape, focus e comportamento atual.
- Fechar ambos deve limpar o estado lógico e não deixar keyboard mode temporário
  ativo.
- Testar a sequência Volume → Calendar → Volume e as variações com click-outside
  e Escape.

### Não fazer

- Não migrar Calendar para layer-shell.
- Não remover a guarda de identidade de fechamento.
- Não criar coordinator genérico para launcher ou aplicações externas.
- Não alterar foco ou keyboard mode de superfícies que não são o popup ativo.

### Critérios de aceite

- Volume e Calendar nunca ficam visivelmente ativos ao mesmo tempo.
- Fechar o popup antigo depois de abrir o novo não fecha o novo.
- Escape fecha o owner que tem teclado; click-outside de Volume não fecha
  Calendar já aberto.
- A janela principal retorna ao estado correto após cada transição.
- Os testes atuais de PopoverState continuam passando ou são ampliados sem
  perder a garantia de simetria.

### Validação

~~~sh
cargo test -p kbar
cargo check -p kbar
~~~

Manual Wayland/Niri: abrir Volume, abrir Calendar, voltar a Volume, usar
Escape/click-outside em cada estado e repetir rapidamente.

### Dependências

T003 e T004; T005/T006 podem ser executadas em paralelo, mas a validação
integrada depende delas.

### Riscos

A surface independente pode emitir hide depois de Calendar assumir o owner.
O coordinator precisa distinguir evento antigo de fechamento do owner atual.

### Notas de implementação

Se a API do coordinator precisar de uma enumeração de owners, manter a mudança
local a apps/kbar/src/ui/popover.rs; não transformar o crate niri em
coordenador de UI.

## T008 — Validação integrada

ID: T008
Título: Validação integrada

### Objetivo

Executar os gates do workspace e validar na sessão Wayland/Niri todos os
comportamentos de popup, slider, outputs, tema e compatibilidade.

### Contexto

GTK4, layer-shell, foco, click-outside, subprocessos e a geometria do
compositor não são cobertos corretamente por unit tests. A constituição exige
testes determinísticos no limite estreito e validação manual para o restante.

### Arquivos prováveis

- Nenhum arquivo de produção é obrigatório nesta task.
- Relatórios de teste ou documentação de execução podem ser adicionados somente
  se o fluxo do repositório já os usar.

### Referências

- AGENTS.md, seção Comandos do projeto e Validação.
- specs/005-volume-popup/spec.md, critérios AC-1 a AC-12.
- docs/architecture/overview.md, limite GTK/layer-shell e testes manuais.
- docs/architecture/design-system.md e ADR-0002 para outputs gerados.

### Implementação esperada

Executar somente as validações automatizadas disponíveis no ambiente atual:

~~~sh
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo check --workspace
cargo build --workspace
cargo run -p kshell-theme-gen -- --check
~~~

Se algum comando não puder ser executado por dependência ausente ou outra
limitação do ambiente, registrar o comando tentado, o motivo da impossibilidade
e a validação que permanece pendente. Uma compilação ou teste unitário nunca é
evidência de sucesso para comportamento visual, foco, layer-shell ou interação
de ponteiro.

### Checklist manual Wayland/Niri

Executar estes itens somente em uma sessão Niri/Wayland utilizável. Não simular
interação gráfica nem marcar um item como aprovado sem execução real. Enquanto a
sessão não estiver disponível, cada item deve permanecer exatamente como:

NÃO EXECUTADO — requer validação manual em sessão Niri/Wayland.

Checklist para validação posterior:

1. abrir popup — NÃO EXECUTADO — requer validação manual em sessão Niri/Wayland.
2. fechar popup — NÃO EXECUTADO — requer validação manual em sessão Niri/Wayland.
3. abrir/fechar repetidamente — NÃO EXECUTADO — requer validação manual em sessão Niri/Wayland.
4. verificar largura do botão/módulo — NÃO EXECUTADO — requer validação manual em sessão Niri/Wayland.
5. verificar posição do botão/módulo — NÃO EXECUTADO — requer validação manual em sessão Niri/Wayland.
6. drag lento — NÃO EXECUTADO — requer validação manual em sessão Niri/Wayland.
7. drag rápido — NÃO EXECUTADO — requer validação manual em sessão Niri/Wayland.
8. clique direto no slider — NÃO EXECUTADO — requer validação manual em sessão Niri/Wayland.
9. scroll para aumentar — NÃO EXECUTADO — requer validação manual em sessão Niri/Wayland.
10. scroll para diminuir — NÃO EXECUTADO — requer validação manual em sessão Niri/Wayland.
11. mute — NÃO EXECUTADO — requer validação manual em sessão Niri/Wayland.
12. unmute — NÃO EXECUTADO — requer validação manual em sessão Niri/Wayland.
13. alteração externa por wpctl com popup fechado — NÃO EXECUTADO — requer validação manual em sessão Niri/Wayland.
14. alteração externa por wpctl com popup aberto — NÃO EXECUTADO — requer validação manual em sessão Niri/Wayland.
15. alteração externa durante drag — NÃO EXECUTADO — requer validação manual em sessão Niri/Wayland.
16. troca de output — NÃO EXECUTADO — requer validação manual em sessão Niri/Wayland.
17. abrir Calendar após Volume — NÃO EXECUTADO — requer validação manual em sessão Niri/Wayland.
18. abrir Volume após Calendar — NÃO EXECUTADO — requer validação manual em sessão Niri/Wayland.
19. click-outside — NÃO EXECUTADO — requer validação manual em sessão Niri/Wayland.
20. Escape — NÃO EXECUTADO — requer validação manual em sessão Niri/Wayland.
21. comportamento com backend indisponível — NÃO EXECUTADO — requer validação manual em sessão Niri/Wayland.

### Não fazer

- Não iniciar T001 ou nenhuma task de backend futuro como parte da validação.
- Não considerar `cargo test` como prova de layer-shell, foco ou geometria.
- Não alterar outputs gerados manualmente para fazer o check passar; corrigir o
  template/generator se uma mudança visual da feature for necessária.

### Critérios de aceite

- Todos os comandos automatizados passam. Qualquer falha mantém T008 aberta e
  deve ser corrigida ou apresentada como bloqueio explícito; não pode ser
  classificada como sucesso por ser atribuída a uma causa externa.
- A matriz manual cobre todos os itens 1–21 e separa o que foi observado em
  Wayland/Niri do que permanece como `NÃO EXECUTADO — requer validação manual em
  sessão Niri/Wayland.`
- Não há regressão observada em Calendar, Kbar, theme, Niri, mute, scroll,
  outputs ou volume externo.
- O relatório final identifica qualquer limitação de backend ou compositor sem
  convertê-la silenciosamente em sucesso.

### Validação

Os próprios comandos acima e a sessão real são a validação desta task. A
conclusão deve conter explicitamente estas três seções:

#### Validação automatizada executada

Listar cada comando executado e seu resultado. Para comandos indisponíveis,
registrar o comando, o motivo e a validação pendente.

#### Validação manual pendente

Listar os itens do checklist ainda não executados, sem inferir comportamento
visual a partir de build ou testes unitários.

#### Validação não executada por limitação do ambiente

Registrar esta seção quando não houver sessão Niri/Wayland utilizável, usando a
mensagem: `NÃO EXECUTADO — requer validação manual em sessão Niri/Wayland.`

Repetir os casos de interação que falharem após a correção dentro do escopo de
005.

### Dependências

T001, T002, T003, T004, T005, T006 e T007.

### Riscos

Uma máquina sem Wayland/Niri não pode provar a posição, keyboard mode,
click-outside ou reflow. Nesses casos, marcar a verificação como não executada,
não inventar evidência e deixar o gate manual pendente.

### Notas de implementação

Se a validação revelar necessidade de backend persistente, OSD, mixer por
aplicação ou redesign, registrar como follow-up (006-audio-service,
007-volume-osd ou feature posterior), sem ampliar 005 durante o fechamento.
