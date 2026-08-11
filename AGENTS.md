# Diretrizes para agentes

KShell é um workspace Cargo em Rust 2021 para um desktop shell Wayland/Niri
construído com GTK4 e `gtk4-layer-shell`.

## Fontes de verdade e fluxo SDD

- `specs/NNN-feature/spec.md` é o contrato de comportamento e a baseline
  retrospectiva da funcionalidade. Atualize-o quando o comportamento mudar.
- `plan.md` é opcional: crie-o somente para uma mudança ativa que atravesse
  pacotes, introduza uma decisão técnica ou exija uma estratégia de validação
  explícita. O plano descreve o delta; não repete a spec.
- `tasks.md` é opcional: use-o quando houver várias tarefas verificáveis em
  andamento. Não crie planos ou listas vazias para baselines já implementadas.
- A constituição define princípios, `docs/architecture/` registra fatos
  estruturais, `docs/decisions/` registra decisões permanentes e a spec é dona
  do comportamento da funcionalidade. Aponte para a fonte de verdade em vez de
  copiar o mesmo requisito.
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
