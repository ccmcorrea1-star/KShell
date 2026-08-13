# Diretrizes para agentes

KLauncher é um aplicativo Rust 2021 para Wayland/Niri construído com GTK4 e
`gtk4-layer-shell`.

## Fontes de verdade e fluxo SDD

- O Spec Kit está configurado para o Codex em `.agents/skills/` e `.specify/`.
- `$speckit-specify <descrição>` cria uma especificação em
  `specs/NNN-feature/spec.md`.
- Use `$speckit-clarify` para ambiguidades antes do planejamento, quando
  necessário.
- Use `$speckit-plan` quando a mudança introduzir uma decisão técnica ou exigir
  uma estratégia de validação explícita.
- Use `$speckit-tasks` quando houver várias tarefas verificáveis.
- Use `$speckit-analyze` após tasks e antes de implementar.
- Use `$speckit-implement` somente depois de spec, plano e tasks estarem
  prontos.
- A constituição em `.specify/memory/constitution.md` define os princípios; a
  spec é dona do comportamento da funcionalidade.
- Documentação futura deve ser escrita em português brasileiro (pt-BR).
  Identificadores técnicos permanecem em inglês.

## Limites de implementação

- O código do aplicativo fica em `apps/klauncher`.
- Use Rust estável, quatro espaços, `rustfmt`, `snake_case`, `UpperCamelCase`
  para tipos e `SCREAMING_SNAKE_CASE` para constantes.
- Mantenha testes unitários junto da implementação. Use `tests/` somente para
  testes que atravessem limites de pacotes.
- Trate arquivos `.desktop`, ambiente, caminhos e saída de comandos como não
  confiáveis. Preserve launch sem shell, limites de argumentos e subprocessos
  com limites explícitos.
- O launcher usa uma `gtk::ApplicationWindow` layer-shell própria porque
  precisa de foco, teclado, geometria, click-outside e lifecycle independentes.
- O estilo em `apps/klauncher/src/ui/style.css` é mantido junto do app.

## Comandos do projeto

Execute os comandos a partir da raiz:

```sh
cargo build --release -p klauncher
cargo run -p klauncher
cargo install --path apps/klauncher
```

O launcher exige Linux, uma sessão Wayland, GTK4 e `gtk4-layer-shell` para
execução manual. O Niri inicia o binário por meio do `PATH`.

## Validação

Antes de entregar uma mudança de código:

```sh
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
```

Mudanças em GTK, layer-shell ou comportamento de serviços do sistema também
exigem uma verificação manual em uma sessão Wayland/Niri adequada quando
disponível.
