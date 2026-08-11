# Funcionalidade 001: launcher de aplicações Klauncher

Status: baseline implementada, documentada retrospectivamente. Este documento
descreve o comportamento atual e não é uma solicitação para adicionar uma nova
funcionalidade ao launcher.

## Classificação das evidências

- **Confirmado:** descoberta, filtragem de desktop entries, localização,
  parsing de `Exec`, ranking fuzzy, comportamento de seleção, configuração de
  layer-shell e launch direto estão implementados em `apps/klauncher/src/` e
  descritos no README existente.
- **Inferido:** `core/` é o limite da lógica determinística da aplicação e
  `ui/` é o limite de apresentação GTK/layer-shell. Isso segue o layout atual
  de módulos e é o limite que mudanças futuras devem preservar.
- **TBD:** ativação de arquivos/URLs, seleção múltipla e um formato público de
  configuração do launcher não são definidos pela implementação atual.

## Objetivo

Fornecer um launcher de aplicações compacto e orientado ao teclado para o
workspace Wayland atual. Ele descobre aplicações gráficas instaladas, cria um
ranking contra uma consulta e inicia a aplicação selecionada sem passar os
metadados desktop por um shell.

## Requisitos e comportamento

### KLAUNCHER-1 — Descobrir aplicações visíveis

O launcher DEVE inspecionar os diretórios de aplicações XDG atuais, incluindo o
diretório de aplicações do usuário e os diretórios derivados de
`XDG_DATA_DIRS`. Quando `XDG_DATA_DIRS` estiver ausente ou vazio, devem ser
usados os diretórios padrão `/usr/local/share/applications` e
`/usr/share/applications`. O fallback do usuário é
`~/.local/share/applications` quando `XDG_DATA_HOME` não for um caminho
absoluto.

A descoberta PODE recursar em subdiretórios, DEVE considerar arquivos `.desktop`
regulares e DEVE eliminar IDs de desktop-file duplicados de acordo com a
precedência de diretórios existente. A lista resultante é ordenada pelo nome
da aplicação.

### KLAUNCHER-2 — Fazer parsing e filtrar desktop entries

Somente entries com `Type=Application` são elegíveis. O launcher DEVE excluir
entries marcadas com `Hidden=true` ou `NoDisplay=true`, entries excluídas por
`OnlyShowIn`/`NotShowIn` para o desktop atual e entries cujo `TryExec` não
resolva para um executável.

Nomes e nomes genéricos DEVEM usar a localidade preferida quando existir uma
chave localizada correspondente, com a chave base como fallback. Ícones PODEM
ser especificados por nome de ícone ou caminho absoluto. Entries malformadas
ou incompletas DEVEM ser ignoradas.

### KLAUNCHER-3 — Fazer parsing de `Exec` sem shell

O launcher DEVE converter um valor `Exec` em um executável mais um vetor de
argumentos ordenado. DEVE preservar os limites de argumentos entre aspas e NÃO
DEVE invocar um shell. O parser atual suporta o comportamento de field codes de
desktop implementado em `core/desktop.rs`, incluindo nome da aplicação,
caminho do desktop-file, expansão de ícone, percent literal e omissão de
payloads de arquivo/URL não suportados. Aspas inválidas, field codes não
suportados, posicionamento inválido de field codes ou mais de um field de
arquivo/URL DEVEM rejeitar a entry.

### KLAUNCHER-4 — Criar ranking dos resultados da consulta

Uma consulta vazia DEVE retornar todas as aplicações carregadas. Uma consulta
não vazia DEVE usar fuzzy matching sem distinção entre maiúsculas e minúsculas
contra `Name` e `GenericName`, quando presentes. Os resultados DEVEM
priorizar scores fuzzy mais fortes, depois o nome da aplicação e, em um empate
completo, a ordem original. A linha selecionada DEVE voltar ao primeiro
resultado sempre que o conjunto de resultados filtrados mudar.

### KLAUNCHER-5 — Apresentar e selecionar resultados

O launcher DEVE apresentar um painel overlay centralizado usando os tokens de
tema compartilhados. Uma linha de resultado contém apenas o ícone e o nome da
aplicação; nomes longos são truncados com reticências. `Up` e `Down` navegam
com wrapping, `Enter` inicia a linha selecionada, um clique no resultado inicia
essa linha, `Esc` fecha o launcher e um clique fora do painel o fecha. Um
conjunto de resultados vazio DEVE mostrar um estado vazio apropriado.

### KLAUNCHER-6 — Iniciar a entry selecionada

O launcher DEVE iniciar o programa analisado diretamente com seu vetor de
argumentos e aplicar o diretório de trabalho da desktop entry quando presente.
Entries de terminal DEVEM usar `$TERMINAL` quando não vazio e fazer fallback
para `kitty`; entries que não são de terminal NÃO DEVEM herdar streams de
terminal e DEVEM ser colocadas em sua própria sessão Unix quando suportado pela
implementação atual.

A superfície layer-shell DEVE usar o namespace existente do launcher e o modo
de teclado exclusivo. `KSHELL_OUTPUT`, quando nomear um connector visível, PODE
selecionar o monitor-alvo; caso contrário, o compositor escolhe a saída.

## Critérios de aceite

| ID | Critério | Evidência |
| --- | --- | --- |
| AC-1 | Desktop entries visíveis e localizadas são carregadas, ordenadas, deduplicadas e entries malformadas são ignoradas. | Testes unitários em `core/desktop.rs`; verificação manual com fixture XDG quando o parser mudar. |
| AC-2 | A expansão de `Exec` preserva argumentos e rejeita formas inseguras/inválidas sem execução por shell. | Testes unitários em `core/desktop.rs` e `core/launch.rs`; ADR-0001. |
| AC-3 | O comportamento de consulta vazia, fuzzy, generic-name, empate e seleção com wrapping corresponde aos requisitos. | Testes unitários em `core/search.rs` e `ui/selection.rs`. |
| AC-4 | Fallback de terminal, diretório de trabalho e comportamento de sessão de processos que não são de terminal permanecem intactos. | Testes unitários em `core/launch.rs`. |
| AC-5 | Controles GTK, tamanho do painel, seleção de saída e ciclo de vida layer-shell funcionam em uma sessão Wayland. | Testes unitários para geometria/copy determinísticos e validação manual Wayland/Niri. |

## Fora do escopo desta baseline

- **TBD:** abrir uma desktop entry com argumentos de arquivo ou URL.
- **TBD:** ações do launcher além do comando `Exec` primário.
- **TBD:** configuração de painel selecionável pelo usuário ou um arquivo de
  configuração formal.
