# Diretrizes para agentes

KShell é um workspace Cargo em Rust 2021 para um desktop shell Wayland/Niri
construído com GTK4 e `gtk4-layer-shell`.

## Fontes de verdade e fluxo SDD

- O Spec Kit está configurado para o Codex em `.agents/skills/` e `.specify/`.
  Os arquivos de `.agents/skills/` são as skills do projeto; os templates,
  scripts, workflows e a constituição ficam em `.specify/`.
- `$speckit-specify <descrição>` cria uma nova especificação em
  `specs/NNN-feature/spec.md`. A pasta `specs/` é criada pela skill quando
  necessário; não recrie specs antigas manualmente.
- Use `$speckit-clarify` para ambiguidades antes do planejamento, quando
  necessário.
- Use `$speckit-plan` somente quando a mudança atravessar pacotes, introduzir
  uma decisão técnica ou exigir uma estratégia de validação explícita. O plano
  fica na pasta da feature e descreve o delta; não repete a spec.
- Use `$speckit-tasks` quando houver várias tarefas verificáveis em andamento.
  Use `$speckit-analyze` após tasks e antes de implementar.
- Use `$speckit-implement` somente depois de a spec, o plano e as tasks estarem
  prontos. Use `$speckit-converge` para avaliar uma implementação existente e
  registrar trabalho restante.
- `$speckit-taskstoissues` só deve ser usado quando o usuário pedir a criação
  de issues no GitHub.
- A constituição em `.specify/memory/constitution.md` define os princípios; a
  spec é dona do comportamento da funcionalidade. Aponte para a fonte de
  verdade em vez de copiar o mesmo requisito.
- Não recrie `docs/` ou documentação arquitetural legada automaticamente.
  Registre decisões e contexto na spec ou no `plan.md`, salvo solicitação
  explícita para criar documentação separada.
- Preserve o comportamento visível e a compatibilidade atuais, salvo mudança
  explícita na spec. Não implemente comportamentos marcados como `TBD`.
- Documentação futura do fluxo SDD deve ser escrita em português brasileiro
  (pt-BR). Identificadores técnicos, comandos, APIs, bibliotecas, classes,
  funções, tipos e variáveis permanecem em inglês.

## Limites de implementação

- Use Rust estável, quatro espaços, `rustfmt`, `snake_case`, `UpperCamelCase`
  para tipos e `SCREAMING_SNAKE_CASE` para constantes.
- Preserve os limites existentes: `apps/klauncher` para core/UI do launcher,
  `apps/kbar` para UI/services da barra, `crates/niri` para a integração Niri,
  `crates/theme` para tokens/templates/rendering e `tools/theme-gen` para a
  geração.
- Em `apps/kbar`, use `GtkPopover` para menus ancorados que dependem da janela
  principal. Quando um painel exigir foco/teclado, geometria, output, camada,
  click-outside ou lifecycle independentes, use uma `gtk::ApplicationWindow`
  layer-shell própria e registre a decisão na spec correspondente. Isso
  não autoriza migrar todos os popups automaticamente.
- Mantenha testes unitários junto da implementação. Use `tests/` somente para
  testes que atravessem limites de pacotes.
- Trate arquivos `.desktop`, ambiente, caminhos e saída de comandos como não
  confiáveis. Preserve launch sem shell, limites de argumentos e subprocessos
  com limites explícitos; não registre dados pessoais sem necessidade.
- Altere CSS/KDL/mockups gerados pelo theme generator quando templates ou
  tokens mudarem; não edite outputs gerados manualmente.

## Comandos do projeto

Execute os comandos a partir da raiz do repositório:

```sh
cargo build --release -p klauncher   # ou: -p kbar
cargo run -p klauncher              # ou: -p kbar
cargo run -p kshell-theme-gen -- --write
```

O launcher e a barra exigem Linux, uma sessão Wayland, GTK4 e
`gtk4-layer-shell` para execução manual. Depois de alterar qualquer uma das
aplicações, instale o binário usado pelo Niri com `cargo install --path apps/klauncher`
ou `cargo install --path apps/kbar`; o Niri inicia os binários por meio do
`PATH`.

Ao alterar uma surface GTK/layer-shell, valide também lifecycle, foco, Escape,
click-outside, monitor/output, ordem entre surfaces e ausência de alteração na
geometria reservada pela barra.

## Validação

Antes de entregar uma mudança de código ou de artefatos gerados, execute:

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
