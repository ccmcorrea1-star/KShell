# KLauncher

KLauncher é um launcher de aplicativos para Wayland e Niri, construído com
Rust, GTK4 e `gtk4-layer-shell`.

## Funcionalidades

- Descoberta de arquivos `.desktop` pelos diretórios XDG.
- Busca fuzzy por nome e nome genérico.
- Seleção por teclado e mouse.
- Execução sem shell, preservando os limites dos argumentos.
- Overlay centralizado com suporte a Escape e clique fora do painel.
- Estilo Gruvbox integrado ao launcher.

## Requisitos

- Linux com uma sessão Wayland.
- Compositor com suporte a layer-shell; Niri é a integração suportada.
- GTK4, `gtk4-layer-shell`, `pkg-config` e Rust estável.

## Compilar e executar

Na raiz do projeto:

```sh
cargo build --release -p klauncher
cargo run -p klauncher
```

Para instalar o binário usado pelo Niri:

```sh
cargo install --path apps/klauncher
```

O Niri localiza `klauncher` pelo `PATH`.

## Integração com Niri

Inclua [`contrib/niri/klauncher.kdl`](contrib/niri/klauncher.kdl) na
configuração principal do Niri:

```kdl
include "/caminho/para/kshell/contrib/niri/klauncher.kdl"
```

O fragmento mantém o atalho `Mod+Space`, configura o namespace
`my-shell-launcher` e aplica os padrões visuais do overlay.

## Testes e validação

```sh
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
```

A constituição em [`.specify/memory/constitution.md`](.specify/memory/constitution.md)
e as diretrizes em [`AGENTS.md`](AGENTS.md) definem o fluxo de desenvolvimento.

## Licença

Este projeto está sob a licença MIT. Consulte [`LICENSE`](LICENSE).
