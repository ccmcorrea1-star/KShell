# Visão geral da arquitetura do KShell

Status: baseline da implementação atual. Fatos marcados como **Confirmado**
são visíveis no código-fonte, manifests, artefatos gerados ou documentação
existente. Afirmações marcadas como **Inferido** descrevem o limite sugerido
por essas evidências. Itens marcados como **TBD** não são definidos pelo
repositório atual.

## Limite do sistema

KShell é um workspace Cargo que contém duas superfícies nativas GTK4/layer-
shell, dois crates reutilizáveis e uma ferramenta de geração. As superfícies
executam em uma sessão Wayland; Niri, dados de aplicações XDG,
PipeWire/WirePlumber, NetworkManager e sysfs de power-supply do Linux são
interfaces externas.

| Pacote ou área | Responsabilidade | Evidência |
| --- | --- | --- |
| `apps/klauncher` | Descobrir e fazer parsing de entries `.desktop`, ordenar resultados, renderizar o launcher e iniciar o comando selecionado | **Confirmado** por `core/`, `ui/` e `main.rs` |
| `apps/kbar` | Compor a barra superior e os popovers; fazer polling de serviços do sistema; encaminhar atualizações de workers para o GTK | **Confirmado** por `app.rs`, `services/` e `ui/` |
| `crates/niri` | Codificar/decodificar o subset JSON Niri suportado, manter estado de workspaces, reconectar o event stream e expor identificadores de compatibilidade | **Confirmado** por `protocol.rs`, `state.rs`, `connection.rs` e `lib.rs` |
| `crates/theme` | Manter tokens visuais, templates incorporados, descoberta de arquivos gerados e rendering seguro específico por consumidor | **Confirmado** por `tokens.rs` e pelos templates |
| `tools/theme-gen` | Expor `--write` e `--check` para o renderer de tema | **Confirmado** por `main.rs` |
| `contrib/niri`, `mockups`, CSS das aplicações | Consumidores gerados e versionados do tema compartilhado | **Confirmado** pelo generator e pelos headers gerados |

A separação de responsabilidades é **Inferida** como o limite de mudança
pretendido: parsing do core e lógica determinística de serviços podem ser
testados sem GTK, enquanto os módulos de UI adaptam esses resultados à
superfície Wayland.

## Fluxos de runtime

### Launcher

1. `klauncher` descobre diretórios de aplicações legíveis usando valores de
   ambiente XDG e home.
2. O parser desktop filtra entries não suportadas e converte `Exec` em um
   vetor de argumentos sem invocar um shell.
3. O GTK renderiza o overlay layer-shell, filtra as entries em memória com o
   módulo de busca fuzzy e controla a seleção por teclado/mouse.
4. A entry selecionada é passada ao módulo de launch, que inicia o programa
   diretamente e opcionalmente envolve entries de terminal com `$TERMINAL` ou
   `kitty`.

### Bar

1. `kbar` cria uma superfície layer-shell superior e reserva a exclusive zone
   da barra.
2. Um worker Niri publica snapshots de workspaces pelo crate compartilhado.
3. Workers de áudio e de sistema lento leem suas fontes externas em
   intervalos limitados e enviam somente atualizações alteradas.
4. O contexto principal do GTK recebe essas atualizações e atualiza workspaces,
   clock, volume, rede, bateria e popovers.

### Geração de tema

1. `kshell-theme` renderiza templates incorporados a partir dos valores de
   tokens canônicos.
2. O generator escreve ou verifica os consumidores GTK/KDL/mockup versionados.
3. Ele atualiza consumidores configurados de terminal, Cava e Fastfetch
   somente quando o executável, a configuração e o import/seção esperados
   estão presentes.

O terceiro fluxo é **Confirmado** na implementação atual. O comportamento
  exato de transaction/rollback para writes de configuração do usuário é
  **TBD**.

## Restrições arquiteturais

- **Limites de comandos sem shell — Confirmado.** Entries `Exec` de desktop e
  comandos de serviços do sistema são passados a `std::process::Command` como
  executável mais argumentos. Isso está registrado no
  [ADR-0001](../decisions/0001-shell-free-desktop-launch.md).
- **Fonte visual compartilhada — Confirmado.** `crates/theme/src/tokens.rs` e
  seus templates são a fonte dos consumidores visuais gerados. Consulte o
  [sistema de design](design-system.md) e o
  [ADR-0002](../decisions/0002-canonical-theme-generation.md).
- **GTK permanece um limite de apresentação — Inferido.** Parsing
  determinístico, ranking, transições de estado e parsing de comandos vivem em
  módulos não-UI e são testados ali; o comportamento Wayland/GTK continua
  sendo uma preocupação de integração manual.
- **Identificadores Niri são dados de compatibilidade — Confirmado.**
  Namespaces compartilhados, nomes de commands e binding padrão vêm de
  `crates/niri`; os IDs das aplicações GTK permanecem nos módulos das
  aplicações. Os valores Niri são compartilhados com os fragments gerados.
  Consulte o [ADR-0003](../decisions/0003-niri-compatibility-identifiers.md).
- **Falhas externas são tratadas de forma best effort — Confirmado.**
  Diretórios ausentes, comandos indisponíveis, baterias ausentes, sockets Niri
  desconectados e displays sem layer-shell suportado não exigem uma nova
  arquitetura de fallback; o código atual retorna estado vazio/desconhecido ou
  um erro claro no limite da superfície.

## Limite dos testes

Testes unitários permanecem junto dos módulos que exercitam. A suíte atual
cobre parsing de desktop e vetores de launch, ranking fuzzy, estado de seleção,
comportamento de protocolo/estado/reconexão Niri, aritmética de clock/calendário,
parsers e agregação de serviços, estado de interação de volume e rendering de
tema. As superfícies GTK/layer-shell e uma sessão Niri ativa exigem validação
manual. O layout atual e a regra para futuros testes entre pacotes estão
registrados em [`tests/README.md`](../../tests/README.md).

## Definições arquiteturais em aberto

Os itens a seguir não são assumidos silenciosamente por esta baseline:

- **TBD:** orquestração de múltiplas instâncias Kbar entre outputs.
- **TBD:** um formato público e estável de configuração para temas ou política
  de output escolhida pelo usuário.
- **TBD:** se a ativação de arquivos e URLs omitida do parsing de `Exec` deve
  ser suportada.
- **TBD:** um harness de testes de integração independente de compositor para
  GTK e Niri.

Qualquer uma dessas decisões deve primeiro atualizar a spec da funcionalidade
correspondente e depois receber um ADR se a escolha for permanente.
