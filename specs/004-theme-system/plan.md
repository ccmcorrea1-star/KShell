# Funcionalidade 004: plano de implementação do sistema de tema

Status: plano de implementação atual para a baseline retrospectiva.

## Abordagem técnica

1. Mantenha tokens e geometria em `crates/theme/src/tokens.rs`.
2. Mantenha sintaxe específica dos consumidores em
   `crates/theme/templates/` e resolva placeholders pelo renderer existente.
3. Mantenha os caminhos de arquivos gerados versionados centralizados no crate
   de tema, para que `--write` e `--check` operem sobre o mesmo conjunto.
4. Mantenha detecção de consumidores opcionais e transformações que preservam
   estrutura no crate de tema; mantenha a CLI em `tools/theme-gen` limitada à
   seleção de comandos e status de saída.
5. Mantenha orientações do sistema de design em
   `docs/architecture/design-system.md`, com comportamento de funcionalidade
   neste diretório e a decisão de geração no ADR-0002.

## Impacto arquitetural

Nenhum generator ou layer de tema novo é necessário. O fluxo atual de
tokens/templates/consumidores é a arquitetura a preservar. Um novo consumidor
deve adicionar um template focado, regra de detecção explícita, testes de
preservação e um critério de aceite antes de alterar outputs gerados.

## Abordagem de validação

- Execute os testes do pacote de tema e os testes completos do workspace.
- Execute `--write` somente quando tokens/templates mudarem; depois execute
  `--check`.
- Inspecione o diff gerado e verifique que nenhuma configuração específica do
  usuário foi incluída no repositório.
- Execute todos os gates de formato, lint e build; nenhuma sessão Wayland é
  necessária para a validação determinística do rendering.

## Hipóteses e definições em aberto

O design de fonte única e a política atual de preservação de consumidores são
**Confirmados**. A ideia de que consumidores futuros devem seguir a mesma
forma de adapter é **Inferida**. Transacionalidade, reload em runtime e um
schema de tema público são **TBD**.
