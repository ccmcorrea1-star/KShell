# ADR-0004: Tratar o painel de volume como surface layer-shell independente

- Status: Aceito
- Escopo: lifecycle, foco, geometria e coordenação do painel de volume da Kbar

## Contexto

O Volume atual é um `GtkPopover` parentado ao item da barra. Esse modelo é
adequado para um menu contextual simples, mas acopla o lifecycle, o foco e o
keyboard mode do painel à surface principal da Kbar.

O comportamento desejado para a funcionalidade 005 é o de um painel de shell:
foco e teclado próprios, fechamento por clique fora, posicionamento no output
da barra, lifecycle independente e nenhuma alteração na geometria reservada
pela barra. Os quatro shells analisados convergem para uma surface própria
para esse tipo de painel: [GPUI Shell](https://github.com/andre-brandao/gpui-shell/blob/main/crates/app/src/panel.rs),
[VibePanel](https://github.com/prankstr/vibepanel/blob/92063a36833ca8eda61a02485f8bfadcc73e1d43/crates/vibepanel/src/widgets/layer_shell_popover.rs),
[DankMaterialShell](https://github.com/AvengeMedia/DankMaterialShell/blob/master/quickshell/Modules/ControlCenter/ControlCenterPopout.qml)
e [Noctalia](https://github.com/noctalia-dev/noctalia/blob/main/src/shell/panel/panel_manager.h).
Essas referências sustentam o padrão, mas a decisão é baseada nos requisitos
locais da Kbar.

## Decisão

O Volume será implementado como uma `gtk::ApplicationWindow` própria, associada
à mesma `gtk::Application` e configurada com `gtk4-layer-shell`.

A surface deverá:

- usar a camada `Top`, exclusive zone zero e namespace dedicado;
- reutilizar o monitor resolvido para a barra;
- ser criada sob demanda e reutilizada após `hide`;
- possuir `KeyboardMode::OnDemand` somente enquanto estiver ativa;
- usar um click catcher mínimo somente para o fechamento por clique fora;
- permanecer coordenada pelo owner lógico compartilhado com Calendar;
- manter Calendar como `GtkPopover` nesta feature;
- preservar `AudioBackend`, `WpctlBackend`, estado do slider e conteúdo visual
  atuais.

Não será criado um framework genérico de surfaces antes de existir uma segunda
consumidora real.

## Alternativas consideradas

### Manter `GtkPopover`

Rejeitada para o painel de Volume porque mantém a dependência da árvore da
barra e não expressa de forma explícita o ownership de foco, teclado, output e
lifecycle exigido pela feature.

### Migrar todos os popups

Rejeitada nesta etapa. Calendar não possui o mesmo delta aprovado e sua
migração aumentaria o write-set sem necessidade.

### Criar uma infraestrutura genérica

Adiada. A primeira implementação deve validar uma surface independente de
Volume; a generalização só será considerada quando houver uma segunda
consumidora com requisitos compatíveis.

## Consequências

- O Volume deixa de depender do parent GTK da barra.
- A janela principal não precisa assumir `OnDemand` por causa do Volume.
- O coordinator precisa tratar owners de naturezas diferentes: surface própria
  para Volume e `GtkPopover` para Calendar.
- Lifecycle, geometria, ordem das surfaces e foco exigem validação manual em
  Wayland/Niri.
- A mudança não altera o backend de áudio, os comandos externos, o tema ou o
  tamanho reservado pela barra.

## Fonte de comportamento

Os requisitos observáveis permanecem em
[`specs/005-volume-popup/spec.md`](../../specs/005-volume-popup/spec.md). A
sequência de implementação e as validações estão em
[`plan.md`](../../specs/005-volume-popup/plan.md) e
[`tasks.md`](../../specs/005-volume-popup/tasks.md).
