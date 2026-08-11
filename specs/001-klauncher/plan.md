# Funcionalidade 001: plano de implementação do Klauncher

Status: plano de implementação atual para a baseline retrospectiva.

## Abordagem técnica

1. Mantenha descoberta de diretórios XDG, parsing de desktop, seleção de
   localidade e expansão de `Exec` em `apps/klauncher/src/core/desktop.rs`.
2. Mantenha o ranking fuzzy em `core/search.rs` e as transições de seleção em
   `ui/selection.rs`, para que ambos possam ser testados sem um compositor em
   execução.
3. Mantenha a construção de processos em `core/launch.rs`; use um executável e
   um vetor de argumentos em vez de texto de shell, preservando o comportamento
   atual de terminal e sessão.
4. Mantenha ciclo de vida GTK/layer-shell, seleção de monitor, carregamento de
   CSS e tratamento de entrada em `ui/`. Consuma geometria e valores visuais
   compartilhados de `crates/theme`.
5. Mantenha identificadores do compositor e o keybinding padrão provenientes
   de `crates/niri` e do template Niri gerado.

## Impacto arquitetural

Nenhum componente arquitetural novo é necessário. A funcionalidade usa a
separação existente entre core/UI, a fonte compartilhada de tema e o crate de
compatibilidade Niri. Uma mudança futura que atravesse esses limites deve
atualizar a spec da funcionalidade e os documentos de arquitetura/ADR somente
quando o contrato global mudar.

## Abordagem de validação

- Use testes unitários colocados junto do código para descoberta, localização,
  parsing de field codes, score de busca, seleção, construção de comandos e
  geometria determinística.
- Use as verificações de formato, testes, lint, build e tema gerado do
  workspace para toda mudança de implementação.
- Exercite manualmente o overlay, foco do teclado, cliques, seleção de saída e
  ciclo de vida do launch dentro de uma sessão Wayland/layer-shell.

## Hipóteses e definições em aberto

Os limites dos módulos e o contrato de interação atual são **Inferidos** a
partir do código e dos mockups existentes. Ativação de arquivos/URLs, ações
adicionais e uma configuração pública do launcher são **TBD**; este plano
intencionalmente não os projeta.
