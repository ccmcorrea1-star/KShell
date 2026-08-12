# Funcionalidade 003: IPC Niri e integração com o compositor

Status: baseline implementada, documentada retrospectivamente. Esta spec cobre
o subset Niri compartilhado atual e os fragments de compositor gerados.

## Classificação das evidências

- **Confirmado:** tratamento de requests/events JSON, transições de estado de
  workspaces, comportamento de reconexão, requests de foco, identificadores de
  compatibilidade, seleção de saída e fragments KDL gerados estão presentes em
  `crates/niri/`, `contrib/niri/` e nas duas aplicações.
- **Inferido:** `crates/niri` é intencionalmente uma camada pequena e tipada de
  compatibilidade, e não um cliente Niri completo. Isso segue o subset de
  protocolo implementado em `protocol.rs`.
- **TBD:** suporte ao protocolo Niri completo, negociação de versão e um
  harness de integração independente do compositor não estão definidos.

## Objetivo

Manter as superfícies KShell sincronizadas com o compositor Niri, preservando a
configuração local existente que inicia Kbar e abre o Klauncher.

## Requisitos e comportamento

### NIRI-1 — Usar o subset de protocolo suportado

A integração DEVE codificar o request de event stream e os requests de foco de
workspace usando o formato atual do protocolo JSON do Niri. DEVE fazer parsing
dos events suportados de mudança de workspaces, ativação e urgência em valores
tipados, aplicar defaults seguros para campos opcionais e ignorar events
desconhecidos ou malformados sem corromper o último estado válido.

### NIRI-2 — Manter estado de workspace consciente da saída

O estado de workspace DEVE manter IDs, índices, nomes, nomes de saída,
flags active/focused, urgência e dados de active-window suportados pelo modelo
atual. Um snapshot completo de workspaces DEVE substituir o snapshot anterior.
Mudanças de ativação DEVEM ser associadas ao workspace/saída relevante e não
DEVEM alterar o workspace ativo de outra saída.

### NIRI-3 — Fazer stream e reconectar com segurança

O event stream DEVE usar `NIRI_SOCKET`. DEVE reconectar depois de um socket
ausente, com falha ou fechado usando o backoff limitado existente, resetar o
backoff depois de uma conexão estável, limpar o estado publicado obsoleto antes
de ressincronizar e parar quando o receiver não aceitar mais atualizações.

O backoff atual começa em 250 ms e é limitado a 2 segundos. Esses valores são
comportamento atual, não uma política de timing generalizada para integrações
futuras.

### NIRI-4 — Focar workspaces por meio do compositor

Cliques em workspaces DEVEM usar um ID identificado quando a barra conseguir
resolvê-lo para a saída selecionada/focada. Caso contrário, a integração DEVE
preservar o request existente por índice one-based e o comportamento no-op para
índice zero.

Requests de foco DEVEM ser enviados diretamente ao socket Niri, sem shell.

### NIRI-5 — Separar IDs GTK dos identificadores de compatibilidade do compositor

Os IDs das aplicações GTK DEVEM usar o namespace do KShell e permanecer
definidos nos módulos das aplicações:

- ID da aplicação launcher: `io.github.ccmcorrea1.kshell.Launcher`;
- ID da aplicação bar: `io.github.ccmcorrea1.kshell.Bar`.

Esses IDs não são referenciados pelos fragments Niri. Os identificadores que
formam o contrato de compatibilidade do compositor DEVEM permanecer
sincronizados entre consumidores nativos e templates gerados. Os valores Niri
compartilhados ficam centralizados em `crates/niri`:

- namespace do launcher: `my-shell-launcher`;
- namespace da bar: `my-shell-bar`;
- command da bar: `kbar`;
- command do launcher: `klauncher`;
- binding do launcher: `Mod+Space`.

Os fragments gerados DEVEM manter startup do Kbar, binding do launcher,
defaults visuais discretos do layout e a regra atual de blur do launcher.
`KSHELL_OUTPUT` PODE direcionar um connector explícito; a política completa de
múltiplas saídas fica fora desta funcionalidade.

## Critérios de aceite

| ID | Critério | Evidência |
| --- | --- | --- |
| AC-1 | Requests/events suportados são codificados e decodificados corretamente, enquanto events desconhecidos não alteram o estado. | Testes unitários em `protocol.rs` e `state.rs`. |
| AC-2 | Snapshots, ativação, urgência, seleção por output focado e IDs de workspace permanecem conscientes da saída. | Testes unitários em `state.rs` e `apps/kbar/src/ui/workspaces.rs`. |
| AC-3 | Conexões ausentes/fechadas usam backoff, limpam estado obsoleto e param quando o receiver é fechado. | Testes unitários em `connection.rs`; verificação de stream em uma sessão Niri. |
| AC-4 | Requests de foco preservam semântica de IPC direto e fallback por índice/ID. | Testes de protocol/connection e verificação manual de clique em workspace. |
| AC-5 | Superfícies nativas e KDL gerado usam os mesmos identificadores Niri de compatibilidade e as mesmas regras atuais, enquanto as aplicações GTK usam IDs no namespace KShell. | Testes de rendering de tema, inspeção dos IDs nos módulos GTK, verificação de tema gerado e verificação manual da configuração Niri. |

## Fora do escopo desta baseline

- **TBD:** cobertura completa do protocolo Niri ou negociação de versão.
- **TBD:** descoberta automática de políticas para múltiplos outputs.
- **TBD:** comportamento para funcionalidades do compositor que não estejam
  representadas pelo subset tipado.
