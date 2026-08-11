# Funcionalidade 003: plano de integração Niri

Status: plano de implementação atual para a baseline retrospectiva.

## Abordagem técnica

1. Mantenha tipos de protocolo e encoding/decoding JSON em
   `crates/niri/src/protocol.rs`.
2. Mantenha transições de estado de workspace com estilo imutável e lookup
   consciente da saída em `crates/niri/src/state.rs`.
3. Mantenha ciclo de vida do socket Unix, backoff de reconexão, reset de estado
   obsoleto e requests de foco em `crates/niri/src/connection.rs`.
4. Mantenha strings de compatibilidade em `crates/niri/src/lib.rs` e consuma-as
   nas aplicações e templates de tema, em vez de duplicar literais.
5. Mantenha configuração opcional do compositor nos fragments KDL gerados
   `contrib/niri/*.kdl`. A configuração Niri principal do usuário continua
   sendo a dona da integração.

## Impacto arquitetural

O crate compartilhado pequeno existente é o limite correto para o
comportamento atual; não é necessário um cliente Niri completo nem uma nova
abstração de IPC. Uma expansão do protocolo deve primeiro estabelecer o
subset de events/actions necessário e seu impacto de compatibilidade na spec
da funcionalidade.

## Abordagem de validação

- Use testes unitários para formatos JSON, events desconhecidos, transições de
  estado e semântica de timing/reset da reconexão.
- Execute as verificações de tema gerado depois de mudanças em identificadores
  ou templates.
- Inclua manualmente os fragments, inicie Kbar, invoque Klauncher e clique em
  workspaces em uma sessão Niri real.

## Hipóteses e definições em aberto

O subset tipado e os identificadores de compatibilidade são **Confirmados**. A
intenção de manter o crate pequeno em vez de completo é **Inferida**. Cobertura
completa do protocolo, negociação de versão e política de múltiplas saídas são
**TBD**.
