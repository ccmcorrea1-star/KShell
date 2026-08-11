# Constituição do KShell

Versão: 1.0

Esta constituição define as regras aplicáveis a todas as funcionalidades. Ela
descreve os princípios de qualidade e compatibilidade do projeto; os comandos
exatos e o fluxo de trabalho dos agentes ficam em `AGENTS.md`.

## Princípio 1 — Especificar o comportamento antes de alterá-lo

Toda mudança de funcionalidade deve ter um diretório correspondente em
`specs/NNN-feature/`. `spec.md` é o contrato de comportamento, `plan.md`
explica a abordagem técnica e o impacto arquitetural, e `tasks.md` divide o
trabalho em ações pequenas e verificáveis. Funcionalidades existentes podem
ser documentadas retrospectivamente, mas o comportamento inferido deve ser
marcado como inferido e escolhas não resolvidas devem ser marcadas como `TBD`.

Toda documentação futura do fluxo Spec-Driven deve ser escrita em português
brasileiro (pt-BR), sem criar documentação bilíngue. Nomes de arquivos e
identificadores técnicos, comandos, APIs, bibliotecas, classes, funções, tipos
e variáveis permanecem em inglês.

## Princípio 2 — Preservar a compatibilidade por padrão

O alvo padrão de implementação é o comportamento visível atual e o contrato
de integração atual. Mudanças nos controles do launcher, nos nomes/namespaces
do Niri, nos artefatos gerados, na propriedade de configurações ou nas
interfaces externas exigem critérios de aceite explícitos. Não introduza uma
arquitetura ou política de fallback nova apenas para fazer uma spec parecer
completa.

## Princípio 3 — Manter explícitos os limites de confiança

Conteúdo de `.desktop`, variáveis de ambiente, caminhos do sistema de
arquivos, saída de subprocessos e mensagens do compositor são entradas não
confiáveis. Analise e valide esses dados no limite, preserve vetores de
argumentos, evite interpretação por shell, limite a execução de comandos
externos e falhe de forma segura ou degrade para um estado desconhecido
explícito quando os dados forem inválidos ou estiverem indisponíveis.

## Princípio 4 — Manter uma única fonte de verdade para a apresentação compartilhada

Tokens visuais, geometria, tipografia e valores de integração gerados
compartilhados pertencem a `crates/theme` e seus templates. Arquivos CSS/KDL/
mockup versionados são consumidores gerados. Arquivos de configuração do
usuário continuam responsáveis pelas configurações não relacionadas ao tema.
Uma mudança visual só está completa quando seus consumidores gerados e testes
de rendering estão consistentes.

## Princípio 5 — Respeitar os limites de pacotes existentes

Mantenha parsing determinístico, ranking, transições de estado e adapters de
serviço testáveis fora do GTK. Mantenha o ciclo de vida e a apresentação
GTK/layer-shell nos módulos de UI das aplicações. Reutilize `crates/niri` para
o comportamento de protocolo/estado Niri e `crates/theme` para rendering
compartilhado, em vez de criar implementações locais paralelas.

## Princípio 6 — Verificar o comportamento no limite confiável mais estreito

Adicione testes focados junto do comportamento que cobrem. Prefira testes
unitários determinísticos para parsing, ranking, estado, construção de
comandos, agregação e rendering. Use validação manual Wayland/Niri para
compositor, layer-shell, GTK e comportamento de serviços externos em execução.
Os critérios de aceite de uma spec devem identificar qual evidência é
automatizada e qual exige uma sessão.

## Princípio 7 — Manter a mudança rastreável

Documentos de funcionalidade são responsáveis pelo comportamento da
funcionalidade. `docs/architecture/` é responsável pela estrutura e pelas
restrições globais. `docs/decisions/` registra decisões permanentes importantes
com contexto e consequências. Não duplique um requisito entre essas camadas;
aponte para o documento de origem.

## Governança

Esta constituição é a baseline global para futuras specs e mudanças de código.
Uma exceção proposta deve ser documentada no plan da funcionalidade afetada e,
quando permanente, em um ADR. A exceção deve explicar seu impacto em
compatibilidade, segurança e validação.
