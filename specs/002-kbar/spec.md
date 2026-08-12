# Funcionalidade 002: barra superior Kbar e serviços de status

Status: baseline implementada, documentada retrospectivamente. Este documento
descreve a barra superior existente e não autoriza novas integrações de
serviços.

## Classificação das evidências

- **Confirmado:** a superfície da barra, cinco slots de workspace, clock/
  calendário, controles de volume, status de rede, status opcional de bateria,
  fluxo de atualizações dos workers e popovers estão implementados em
  `apps/kbar/src/`.
- **Inferido:** `services/` é o limite de adapters e workers, enquanto `ui/`
  cuida da renderização e da interação GTK. Isso segue o layout atual do
  pacote.
- **TBD:** orquestração de múltiplas barras, uma API persistente de serviços e
  módulos de status definidos pelo usuário não estão definidos.

## Objetivo

Fornecer uma barra superior compacta e plana que exponha o estado dos workspaces e
alguns status locais selecionados, continuando utilizável quando serviços
externos individuais estiverem ausentes.

## Requisitos e comportamento

### KBAR-1 — Criar a superfície da barra

Kbar DEVE criar uma superfície GTK4 layer-shell no topo da saída selecionada,
usar o namespace existente da barra e reservar a exclusive zone atual. A
superfície DEVE ser não interativa para foco de teclado por padrão e usar os
tokens de tema compartilhados e o CSS gerado.

`KSHELL_OUTPUT`, quando nomear um connector visível, DEVE ser usado para
seleção de monitor e para o contexto de saída do workspace Niri. Sem ele, o
compositor e o foco Niri atual determinam o contexto.

### KBAR-2 — Exibir e focar workspaces

A barra DEVE exibir cinco slots visuais de workspace. A quantidade de slots é
uma escolha de UI e NÃO DEVE ser tratada como a quantidade de workspaces do
compositor. O estado de workspace vem da integração Niri compartilhada. O slot
ativo é associado à saída selecionada ou focada.

Um clique DEVE focar o ID do workspace correspondente quando o estado atual
identificar um de forma não ambígua. DEVE fazer fallback para a requisição
existente por índice quando nenhum ID utilizável estiver disponível.

### KBAR-3 — Exibir clock e calendário

O centro da barra DEVE exibir a data e a hora locais atuais no formato
abreviado em português atual. O clock DEVE ser atualizado nos limites de minuto.
Um clique DEVE abrir o popover do calendário, que mostra uma grade de mês com
42 células, destaca o dia atual, oferece navegação para o mês anterior/próximo
e fecha com Escape.

### KBAR-4 — Informar status de rede e bateria

O status de rede DEVE usar `nmcli` quando disponível e fazer fallback para a
presença de uma rota padrão por meio de `ip route show default`. A barra DEVE
atualizar o ícone e o tooltip de rede quando o estado mudar.

O status de bateria DEVE ler entries de power-supply do Linux, calcular a
média das capacidades válidas, marcar baterias charging/full como carregando e
ocultar o módulo de bateria quando nenhuma bateria estiver disponível. Os
valores de capacidade DEVEM ser limitados a uma porcentagem.

### KBAR-5 — Informar e controlar o áudio

O volume DEVE ser lido e controlado pelo backend `wpctl` existente. A barra
DEVE expor a porcentagem atual e o estado de mute, suportar ajustes de 5% com a
roda do mouse, acesso ao painel de volume com clique esquerdo e mute com clique
do meio. O painel DEVE fornecer slider, controle de mute e lista de saídas
disponíveis com a saída ativa marcada. Selecionar uma saída DEVE usar
`wpctl set-default`. O lifecycle, o foco e a surface independente do painel são
detalhados na [funcionalidade 005](../005-volume-popup/spec.md).

O worker DEVE atualizar o volume com frequência, atualizar dispositivos de
saída com menor frequência, agrupar ações `Set` contíguas do slider, preservar
ordem em torno das ações de mute/saída e reconciliar o estado otimista do
slider com uma leitura posterior do backend.

### KBAR-6 — Limitar trabalho externo e publicar estado alterado

Comandos de serviços DEVEM executar sem shell, com execução e tratamento de
saída limitados. Workers em segundo plano DEVEM publicar o estado inicial e as
alterações seguintes, eliminando snapshots idênticos de áudio e do sistema
lento antes de chegarem ao event loop GTK. Um comando ausente ou uma fonte
ilegível DEVE produzir um status desconhecido ou indisponível, sem bloquear a
UI indefinidamente.

## Critérios de aceite

| ID | Critério | Evidência |
| --- | --- | --- |
| AC-1 | A barra usa a geometria layer-shell, o namespace, o output de tema e o comportamento de seleção de saída existentes. | `app.rs` e código de UI; verificação manual em uma sessão Wayland. |
| AC-2 | O estado ativo e o alvo de clique de workspace são conscientes da saída e preservam o contrato de cinco slots. | Testes unitários em `ui/workspaces.rs`; testes de estado Niri. |
| AC-3 | Formatação do clock em português, alinhamento por minuto, aritmética de calendário e navegação do popover comportam-se conforme especificado. | Testes unitários em `clock.rs` e `calendar.rs`; verificação manual GTK. |
| AC-4 | Fallback de rede, parsing/visibilidade de bateria e deduplicação de status permanecem limitados e orientados a mudanças. | Testes unitários em `services/network.rs`, `services/battery.rs` e `services/mod.rs`. |
| AC-5 | Parsing de volume, parsing de saídas, ordem de ações, sincronização do slider e estados de interação preservam o comportamento atual. | Testes unitários em `services/audio.rs` e `ui/volume.rs`; verificação manual PipeWire. |

## Fora do escopo desta baseline

- **TBD:** APIs persistentes orientadas a eventos para NetworkManager ou
  PipeWire.
- **TBD:** exibir mais de uma instância de Kbar como um sistema coordenado de
  múltiplas saídas.
- **TBD:** módulos, intervalos de polling ou política de status configuráveis
  pelo usuário.
