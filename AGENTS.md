# Diretrizes para agentes

KShell é um workspace Cargo em Rust 2021 para um desktop shell Wayland/Niri
construído com GTK4 e `gtk4-layer-shell`.

## Regras estáveis do repositório

- Trate `specs/NNN-feature/spec.md` como a baseline de comportamento de uma
  funcionalidade existente. Atualize `plan.md` e `tasks.md` quando uma mudança
  solicitada afetar essa funcionalidade.
- Mantenha os princípios globais em `.specify/memory/constitution.md`, os
  fatos arquiteturais em `docs/architecture/` e as decisões permanentes em
  `docs/decisions/`. Não copie o mesmo requisito em vários documentos.
- Preserve o comportamento visível para o usuário e a compatibilidade atuais,
  salvo quando uma spec de funcionalidade alterar isso explicitamente. Não
  implemente comportamentos marcados como `TBD`.
- Toda documentação futura do fluxo Spec-Driven deve ser escrita em português
  brasileiro (pt-BR), sem criar documentação bilíngue. Mantenha em inglês os
  identificadores técnicos, comandos, APIs, bibliotecas, classes, funções,
  tipos e variáveis.
- Use Rust estável, indentação de quatro espaços, `rustfmt`, `snake_case` para
  funções/variáveis/módulos, `UpperCamelCase` para tipos e
  `SCREAMING_SNAKE_CASE` para constantes.
- Mantenha a lógica das aplicações separada pelos limites de módulos
  existentes: `apps/klauncher` é responsável pelo core/UI do launcher,
  `apps/kbar` pelo UI/services da barra, `crates/niri` pela integração Niri
  reutilizável, `crates/theme` por tokens/templates/rendering compartilhados e
  `tools/theme-gen` pela geração.
- Mantenha os testes unitários junto da implementação que cobrem. Use
  `tests/` apenas para testes que realmente atravessem limites de pacotes; não
  mova testes apenas para atender ao layout SDD.
- Trate arquivos `.desktop`, variáveis de ambiente, saída de comandos e
  caminhos como não confiáveis. Preserve o modelo de launch sem shell, os
  limites dos argumentos e o comportamento de subprocessos com limites
  explícitos. Não registre argumentos de comandos ou caminhos pessoais sem
  necessidade.
- Artefatos CSS/KDL/mockup gerados devem ser alterados pelo theme generator
  quando seus templates ou tokens mudarem; não edite outputs gerados
  manualmente.

## Comandos do projeto

Execute os comandos a partir da raiz do repositório:

```sh
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
cargo build --release -p klauncher   # ou: -p kbar
cargo run -p klauncher              # ou: -p kbar
cargo run -p kshell-theme-gen -- --check
cargo run -p kshell-theme-gen -- --write
```

O launcher e a barra exigem Linux, uma sessão Wayland, GTK4 e
`gtk4-layer-shell` para execução manual. Depois de alterar qualquer uma das
aplicações, instale o binário usado pelo Niri com `cargo install --path apps/klauncher`
ou `cargo install --path apps/kbar`; o Niri inicia os binários por meio do
`PATH`.

## Validação obrigatória

Antes de entregar uma mudança de código ou de artefatos gerados, execute as
verificações de formatação, testes do workspace, lint, build e tema gerado:

```sh
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
cargo run -p kshell-theme-gen -- --check
```

Execute `cargo check --workspace` quando a mudança afetar compilação ou
dependências Rust. Mudanças em GTK, layer-shell, Niri ou comportamento de
serviços do sistema também exigem uma verificação manual em uma sessão
Wayland/Niri adequada quando disponível.
