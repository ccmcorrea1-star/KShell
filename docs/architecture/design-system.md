# Sistema de design: Gruvbox do KShell

Status: baseline visual da implementação atual. As regras abaixo são
confirmadas pelos templates, tokens, mockups e código GTK existentes. A
funcionalidade 005 altera a surface do painel de Volume, mas não aprova uma
segunda linguagem visual.

Klauncher é uma superfície compacta e orientada ao teclado para escolher uma
aplicação instalada sem sair do workspace Wayland atual. Sua referência visual
é o launcher nativo e seu mockup correspondente em
`mockups/launcher-designs.html`.

A aplicação selecionada é marcada por uma faixa estreita e discreta à
esquerda. Todo o restante permanece silencioso e legível para que a consulta e
a seleção sejam a única hierarquia que a pessoa precise ler.

Kbar estende o mesmo sistema para uma superfície superior baixa de `32px`. Sua
referência aprovada é `mockups/bar-design.html`: cinco controles de workspace à
esquerda, data/hora em português centralizada pela viewport em vez de pelo
conteúdo vizinho e status do sistema conduzido por ícones à direita. A
superfície usa o mesmo canvas, borda estrutural, tipografia mono, raio de `2px`
e vocabulário de espaçamento do Klauncher. Ela não tem paleta, elevação, blur,
sombra, pill ou tratamento HUD independente.

O módulo de status de volume é uma extensão intencional dessa superfície
discreta: seu ícone e porcentagem permanecem visíveis na barra, enquanto o
painel compacto expõe somente o controle baseado em PipeWire e uma lista direta
de saídas com o dispositivo ativo marcado. A surface independente aprovada para
o painel não cria estado de áudio, HUD, glow ou controles decorativos separados.

## Tokens canônicos e consumidores

`crates/theme/src/tokens.rs` é a única fonte canônica de cores, entradas de
paleta semântica e ANSI, tipografia, espaçamento, raios, bordas e geometria
aprovada do launcher. Ele define as superfícies Gruvbox neutras usadas pelo
launcher e a paleta ANSI consumida pelos terminais configurados.

O generator renderiza estes templates:

- `crates/theme/templates/style.css` → `apps/klauncher/src/ui/style.css` para GTK
- `crates/theme/templates/kbar.css` → `apps/kbar/src/ui/style.css` para GTK
- `crates/theme/templates/kbar.kdl` → `contrib/niri/kbar.kdl` para autostart do Niri
- `crates/theme/templates/klauncher.kdl` → `contrib/niri/klauncher.kdl` para o Niri
- `crates/theme/templates/theme.css` → `mockups/theme.css` para o mockup do browser
- `crates/theme/templates/kitty.conf` → arquivo de tema Kitty importado ativo
- `crates/theme/templates/alacritty.toml` → arquivo de cores Alacritty importado ativo
- `crates/theme/templates/foot.ini` → arquivo de cores Foot importado ativo, quando Foot está instalado e configurado
- `crates/theme/templates/cava.ini` → seção `[color]` ativa da configuração do Cava
- `crates/theme/templates/fastfetch.jsonc` → campos de cor da configuração ativa do Fastfetch

O mockup carrega `theme.css`; não adicione uma segunda fonte de valores
visuais a ele. Os arquivos gerados carregam um header e não devem ser editados
diretamente.

Consumidores de terminal, visualizador e informações do sistema são detectados antes
da escrita: o renderer exige o executável, a configuração do usuário e um
tema importado ou seção de cores ativa existentes. Ele não cria nem modifica
um consumidor que esteja presente somente como executável ou configuração
órfã. A configuração principal do terminal continua responsável por fontes,
shell, atalhos e outros comportamentos. Fastfetch preserva a origem do logo,
módulos e layout enquanto substitui somente valores de cor. O único ajuste
fora da paleta é forçar opacity `1.0` em uma janela existente do Alacritty para
que a regra global de ausência de transparência seja efetiva.

Depois de alterar tokens ou um template, execute:

```sh
cargo run -p kshell-theme-gen -- --write
cargo run -p kshell-theme-gen -- --check
```

O teste de tema colocado junto do código também verifica que todo template é
resolvido e que os outputs versionados correspondem ao renderer.

## Tipografia e geometria

Use a família mono canônica no tamanho e altura de linha regulares da UI. Consulta,
prompt, placeholder, nomes de linhas, estado vazio, mockup e implementação GTK
nativa compartilham esses tokens. Configurações de fonte do terminal continuam
sob responsabilidade de cada configuração de terminal.

O launcher mantém sua geometria aprovada de painel fixo quando há espaço na
tela; em telas menores, a implementação nativa apenas o limita para preservar
a margem canônica. Cabeçalho, inset da lista, ritmo das linhas, tamanho do ícone
e gap entre ícone e nome são tokens de geometria nomeados. Nomes de aplicações são
truncados com reticências, sem alterar esse ritmo.

Cada linha de aplicação renderiza somente:

```text
[icon]  Nome da aplicação
```

Ícones desktop reais preservam suas cores de origem. Eles são reconhecedores de
aplicações, não acentos da interface.

## Estados e profundidade

Linhas ociosas são transparentes sobre a superfície. Hover usa a superfície
neutra elevada. Seleção usa a superfície neutra selecionada mais a faixa
estrutural à esquerda e permanece visível independentemente do hover. Focus
usa a borda estrutural sem glow. Conteúdo desabilitado usa o token de texto
desabilitado sobre uma superfície neutra.

O painel é plano: seu contorno estrutural e o contraste contra o canvas
estabelecem sua borda. Não use sombras, gradientes, efeitos de vidro,
tratamentos HUD, cores brilhantes de focus ou decoração que não ajude na busca
ou seleção. Quando o launcher abre, somente o campo voltado ao desktop recebe
um dim preto contido e blur do compositor; o painel do launcher permanece
sólido e nítido.

Fundos, foregrounds, cursores e seleções de terminais usam os tokens globais
de superfície. A paleta Gruvbox ANSI completa é reservada para saída de
comandos, syntax highlighting, links e estados semânticos de terminal; o
chrome do terminal permanece neutro.

Cava usa o mesmo fundo e foreground globais com uma rampa Gruvbox neutra
contida para as barras. Sua visualização de áudio não introduz cores ANSI ou
cores decorativas.

## Responsabilidade das funcionalidades

As regras visuais vivem neste documento de arquitetura. Requisitos de
interação e questões em aberto de cada funcionalidade vivem em
[`001-klauncher`](../../specs/001-klauncher/spec.md),
[`002-kbar`](../../specs/002-kbar/spec.md) e
[`004-theme-system`](../../specs/004-theme-system/spec.md), com o lifecycle da
surface de Volume detalhado em
[`005-volume-popup`](../../specs/005-volume-popup/spec.md). O limite de
geração é uma decisão arquitetural aceita no
[`ADR-0002`](../decisions/0002-canonical-theme-generation.md).
