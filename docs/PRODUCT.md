# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

Este registro descreve a superfície de mockup HTML estático do Klauncher. O produto representado pelo mockup é um aplicativo nativo para Linux.

O mockup em `mockups/launcher-designs.html` é a referência visual canônica do launcher nativo: ambos usam o mesmo painel compacto Gruvbox de `520 × 300px`.

## Users

A pessoa principal é alguém que trabalha em Linux, em uma sessão Wayland com prioridade para o Niri, e quer abrir uma aplicação sem sair do espaço de trabalho atual.

## Product Purpose

Klauncher permite encontrar e iniciar rapidamente uma aplicação instalada. O sucesso consiste em chegar à aplicação desejada com uma interação curta, orientada pelo teclado, sem obrigar a pessoa a percorrer um menu desktop.

## Positioning

Klauncher lê os arquivos `.desktop` do desktop, classifica as aplicações com busca fuzzy e inicia a aplicação escolhida sem invocar um shell. Seu posicionamento é o de uma sobreposição pequena e direta, não o de um shell desktop amplo ou de um catálogo de aplicações.

## Operating Context

O contexto prioritário é uma sessão Wayland usando Niri. A integração documentada pode abrir o launcher com `Mod+Space`; a pessoa digita uma Launcher Query, navega pela Application Selection com Up e Down, inicia com Enter ou fecha com Esc.

## Capabilities and Constraints

- O mockup HTML é estático; seus controles simulam a busca, a navegação e a seleção, mas não iniciam aplicações.
- A implementação nativa descobre aplicações nos diretórios XDG padrão.
- A busca cobre nomes de aplicações e nomes genéricos com classificação fuzzy.
- Desktop Entries podem fornecer ícones nomeados ou caminhos absolutos para ícones.
- A inicialização preserva os limites dos argumentos de `Exec` e não invoca um shell.
- Aplicações que exigem um terminal usam `$TERMINAL` e, quando a variável não está definida, recorrem a `kitty`.
- A implementação nativa usa Rust 2021, GTK4 e `gtk4-layer-shell`.
- A implementação nativa exige um compositor Wayland com suporte a layer-shell.
- A terminologia do produto está definida em `docs/CONTEXT.md`: Desktop Entry, Application, Application Selection e Launcher Query.
- A escolha definitiva do terminal permanece em aberto.
- A decisão sobre ampliar o launcher para uma command palette permanece em aberto.

## Brand Commitments

- O nome do produto é Klauncher.
- A interação é concisa, orientada pelo teclado e adequada a uma sobreposição sempre disponível.
- A aparência é Gruvbox, escura e plana: `#1d2021` no cabeçalho, `#282828` no painel, `#3c3836` na seleção e `#ebdbb2` no texto.
- Cada Application mostra somente ícone e nome; o indicador esquerdo discreto comunica a seleção.
- A interface não usa cores vibrantes, HUD, glow, gradientes, sombras ou controles decorativos.

## Evidence on Hand

- `README.md` documenta o comportamento atual do produto e seus requisitos operacionais.
- `docs/CONTEXT.md` define a terminologia aprovada da interface.
- `apps/klauncher/src/ui/gtk.rs` contém as dimensões atuais da sobreposição e a ligação dos controles.
- `apps/klauncher/src/ui/style.css` contém o estilo GTK atual.
- `mockups/launcher-designs.html` é o mockup HTML estático usado para iteração visual.
- Não há ativos de marca, alegações de clientes, depoimentos ou imagens de produto fornecidos para serem fabricados ou sugeridos.

## Product Principles

- Abrir a aplicação desejada com o mínimo de interrupção.
- Tornar o estado do teclado visível sem exigir instruções.
- Preservar os limites dos comandos da pessoa e as convenções do desktop.
- Tratar os Desktop Entries instalados como fonte de verdade das aplicações.
