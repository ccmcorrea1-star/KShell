---
target: mockups/launcher-designs.html
total_score: 21
max_score: 40
na_heuristics: 
p0_count: 0
p1_count: 3
timestamp: 2026-08-09T01-48-41Z
slug: mockups-launcher-designs-html
---
Method: dual-agent (A: ses_01bd47b33fferxQprF9ChUE1N2 · B: ses_01bcda6ddffeLByOiu9QgaTpQG)

## Saúde do Design

| # | Heurística | Nota | Problema principal |
|---|---|---:|---|
| 1 | Visibilidade do estado do sistema | 2/4 | Contador, estado `READY` e seleção são visíveis, mas não há feedback de execução ou fechamento. |
| 2 | Correspondência com o mundo real | 3/4 | O fluxo de busca é familiar, mas mistura inglês, termos técnicos e categorias sem uma decisão linguística clara. |
| 3 | Controle e liberdade do usuário | 1/4 | `Esc` promete fechar, mas apenas limpa a busca; a saída demonstrada não corresponde à promessa. |
| 4 | Consistência e padrões | 2/4 | A composição é consistente, mas `enter` e `02/03/04` têm semânticas diferentes e o modelo de foco muda após um clique. |
| 5 | Prevenção de erros | 2/4 | O formulário não submete acidentalmente, mas seleção obsoleta e estados vazios não são tratados completamente. |
| 6 | Reconhecimento em vez de memorização | 3/4 | Aplicações, categorias, contador e instruções ficam visíveis, mas os hints numéricos exigem inferência. |
| 7 | Flexibilidade e eficiência | 2/4 | Há mouse e teclado, mas as setas dependem do foco no input e não há aceleradores adicionais. |
| 8 | Estética e design minimalista | 3/4 | O launcher é disciplinado, mas a moldura editorial adiciona informação à tarefa operacional. |
| 9 | Reconhecer, diagnosticar e recuperar erros | 1/4 | Existe estado vazio, porém sem orientação de recuperação e com rodapé mantendo a seleção anterior. |
| 10 | Ajuda e documentação | 2/4 | Há instruções inline, mas não há ajuda contextual para foco, `Enter`, `Esc` ou ausência de resultados. |
| **Total** |  | **21/40** | **Aceitável; melhorias significativas ainda são necessárias.** |

## Veredito de Especificidade

O núcleo é específico para Klauncher, mas a camada visual ainda é parcialmente intercambiável.

**Avaliação de design:** o painel fixo de `600 x 380px`, o stage de medição, a seleção única, o foco inicial, a busca e as referências a `KLAUNCHER`, `WAYLAND` e `NIRI` ancoram o trabalho no produto. Porém, “Void”, preto terminal, grid de medição e metadados técnicos poderiam ser aplicados quase sem alteração a rofi, Raycast ou outro launcher. Falta um detalhe de marca mais proprietário além do nome e do contexto técnico.

**Varredura determinística:** o detector encontrou 15 achados consultivos em `mockups/launcher-designs.html`:

- 1 alerta de grid decorativo na linha 174.
- 13 alertas de tamanhos tipográficos fora da escala interpretada a partir do `DESIGN.md`, nas linhas 72, 104, 118, 249, 262, 388, 418, 425, 451, 464, 476, 478 e 479.
- 1 alerta de cor não documentada, `rgba(255, 255, 255, 0.7)`, na linha 187.

O alerta da grade é um falso positivo provável: o `DESIGN.md` define explicitamente um stage de medição, e o próprio mockup o usa como contexto de preview. Os valores tipográficos e a opacidade devem ser verificados individualmente; alguns podem ser exceções intencionais, mas hoje não estão registrados de forma consistente no sistema. O detector não capturou os problemas comportamentais de foco, `Enter`, `Esc` e seleção obsoleta.

Não houve evidência de navegador, screenshot ou overlay porque nenhuma ferramenta de navegador estava disponível nesta sessão.

## Impressão Geral

A direção é forte como artefato editorial e comunica bem uma superfície de launcher calma, técnica e keyboard-first. Ainda não é uma validação confiável da experiência porque os dois comandos decisivos, `Enter` e `Esc`, prometem um comportamento que o JavaScript não demonstra. A maior oportunidade é fechar essa lacuna comportamental antes de adicionar mais acabamento visual.

## O Que Está Funcionando

- A seleção invertida é a única mudança de contraste forte na lista, obedecendo à regra central do `DESIGN.md`.
- A composição compacta, o painel fixo, as referências a Wayland/Niri e o foco inicial comunicam diretamente o contexto do Klauncher.
- Cada aplicação tem marca, nome, categoria e estado de seleção em posições previsíveis, o que facilita a varredura rápida.

## Problemas Prioritários

### [P1] `Enter` e `Esc` prometem um fluxo que o mockup não demonstra

**Por que importa:** são os dois momentos decisivos da interação; a inconsistência destrói confiança no fim da jornada.

**Fix:** simular explicitamente o fechamento e o início da aplicação, ou declarar no painel que se trata apenas de uma prévia e remover a promessa de fechamento. O comportamento real de GTK/Niri permanece fora do escopo deste mockup.

**Suggested command:** `$impeccable harden mockups/launcher-designs.html`

### [P1] O modelo keyboard-first depende excessivamente do foco no input

**Por que importa:** depois de clicar em uma linha, as setas deixam de alterar a seleção porque o listener de teclado existe apenas no input (`launcher-designs.html:645-664`).

**Fix:** adotar um único modelo de foco para input e lista, mantendo o foco no campo ou usando foco roving/`aria-activedescendant`; `Enter`, setas e `Esc` devem funcionar a partir de qualquer foco válido.

**Suggested command:** `$impeccable audit mockups/launcher-designs.html`

### [P1] O estado sem resultados mantém uma seleção obsoleta

**Por que importa:** uma busca sem correspondência mostra `no applications found`, mas o rodapé continua indicando `Visual Studio Code`, contradizendo o estado atual (`launcher-designs.html:624-641`).

**Fix:** limpar seleção e rodapé quando `visibleRows` chegar a zero, anunciar o termo pesquisado e oferecer uma recuperação explícita, como limpar a busca.

**Suggested command:** `$impeccable harden mockups/launcher-designs.html`

### [P2] A amostra não prova a busca real nem a escala de aplicações instaladas

**Por que importa:** há apenas quatro linhas, a lista usa `overflow: hidden` e a filtragem usa `includes`; isso não demonstra ranking fuzzy, rolagem ou comportamento com muitos Desktop Entries.

**Fix:** incluir estado com mais resultados, lista rolável, nomes longos e ranking fuzzy representativo. A implementação fuzzy nativa permanece fora do escopo deste arquivo.

**Suggested command:** `$impeccable harden mockups/launcher-designs.html`

### [P2] A moldura editorial e a voz linguística competem com o launcher

**Por que importa:** a página está em `pt-BR`, mas instruções, categorias e metadados permanecem em inglês; há também textos sem acentuação como `superficie`, `ruido`, `selecao` e `nao`. O conteúdo editorial aparece antes da operação principal.

**Fix:** escolher PT-BR ou inglês técnico de forma consistente, corrigir a microcopy e colocar o painel no primeiro foco da página sem abandonar a direção Void.

**Suggested command:** `$impeccable clarify mockups/launcher-designs.html`

## Carga Cognitiva

| Item | Status | Observação |
|---|---|---|
| Foco único | Falha na página completa | Título, faixa de especificações, princípios, ficha técnica e rodapé cercam a tarefa. No painel isolado, passa. |
| Chunking | Falha na página completa | A ficha técnica expõe seis pares de implementação para quem veio operar. |
| Agrupamento | Passa | Cabeçalho, busca, lista e rodapé têm separação clara. |
| Hierarquia visual | Passa | Input e seleção invertida dominam corretamente dentro do launcher. |
| Uma coisa por vez | Passa | O fluxo é busca, navegação e seleção, sem decisão secundária. |
| Escolhas mínimas | Passa | Há quatro aplicações visíveis, dentro do limite de decisão. |
| Memória de trabalho | Passa | A aplicação selecionada permanece identificável na linha e no rodapé. |
| Divulgação progressiva | Passa | Detalhes técnicos ficam abaixo do stage. |

**Total:** 2 falhas de 8 na superfície completa. No painel nativo isolado, 0 falhas de 8.

## Jornada Emocional

**Entrada:** a estética mineral e `WAYLAND / READY` transmitem precisão, mas a página começa com “Void.” e conteúdo de apresentação antes de chegar ao launcher.

**Busca:** o campo, o prompt `>` e o contador reduzem ruído. A fricção vem do placeholder em inglês, da busca por substring em vez da busca fuzzy descrita no produto e do estado vazio sem orientação acionável.

**Seleção:** a inversão branco/preto torna a seleção clara; nome, categoria e rodapé reforçam a escolha. Clicar em uma linha desloca o foco e quebra a navegação por setas. Os marcadores `FI` repetidos diferenciam pouco Firefox e Files.

**Confirmação e saída:** este é o principal vale emocional. `enter` sugere início e `esc to close` sugere fechamento, mas o JavaScript não executa nenhum dos dois. O mockup termina sem confirmação ou sensação de conclusão.

## Persona Red Flags

**Alex, power user:** encontra o input focado, mas perde eficiência ao clicar em uma linha e continuar com as setas. `Enter` não inicia nada e `Esc` não fecha (`launcher-designs.html:645-670`). A ausência de demonstração de fuzzy ranking reduz a credibilidade para quem espera busca rápida.

**Sam, usuário com necessidades de acessibilidade:** o label oculto do input e o `:focus-visible` são bons sinais (`launcher-designs.html:41-56`). Porém, o `listbox` contém botões com `role="option"` sem coordenação clara de foco (`launcher-designs.html:542-562`). `aria-selected` muda, mas não há anúncio suficiente de mudança de seleção ou de erro.

**Riley, pessoa que testa limites:** uma busca sem correspondência deixa o rodapé com a aplicação anterior. Se a lista crescer, `overflow: hidden` pode ocultar resultados e a seleção pode sair da área visível. Não há recuperação explícita para esses estados.

## Observações Menores

- `#F1F1EB` na ficha técnica diverge do foreground definido como `#F0F0EA`.
- Os hints `enter`, `02`, `03` e `04` não têm semântica explicada de forma uniforme.
- O texto secundário pequeno usa `#777A75`; verifique contraste explicitamente para 10px.
- No mobile, o stage preserva `600px` e cria rolagem horizontal, mas não indica visualmente que a área é rolável.
- O foco automático é correto para a sobreposição, mas inesperado para uma página de apresentação.
- Os marcadores `FI` de Firefox e Files são visualmente idênticos.

## Perguntas a Considerar

1. O mockup quer validar uma interação real ou apenas vender uma direção visual? Se for interação, por que `Enter` e `Esc` não têm estados simulados?
2. O primeiro viewport deve apresentar a tese “Void.” ou permitir operar o launcher imediatamente?
3. Quatro aplicações são uma escolha estética deliberada ou estão escondendo a principal prova do produto: ranking fuzzy sobre uma lista real?
4. A linguagem operacional deve ser PT-BR, inglês técnico ou uma combinação formalmente definida?
