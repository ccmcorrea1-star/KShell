# ADR-0001: Iniciar desktop entries sem shell

- Status: Aceito
- Escopo: Klauncher e comandos limitados de serviços do sistema

## Contexto

Valores `Exec` de desktop e caminhos derivados do ambiente são entradas
externas. Passar o valor completo para um shell tornaria o quoting ambíguo e
permitiria que a sintaxe do shell se tornasse comportamento executável. O
launcher também precisa manter os limites de argumentos da desktop entry.

## Decisão

Fazer parsing do valor `Exec` em um executável e um vetor de argumentos.
Iniciá-lo com `std::process::Command`, nunca com `sh -c` ou um shell
equivalente. Entries de terminal usam o programa de terminal configurado como
um executável separado e passam o vetor da aplicação analisada como argumentos.
O mesmo limite sem shell se aplica aos comandos de serviço `wpctl`, `nmcli`,
`ip` e outros do Kbar.

## Consequências

- Quoting e tratamento de field codes são explícitos e testáveis.
- Metacaracteres de shell presentes em dados desktop não são interpretados como
  comandos.
- Limites de argumentos e comportamento de diretório de trabalho permanecem
  visíveis nos testes unitários.
- Scripts de shell embutidos em um valor `Exec` não são um caminho de launch
  suportado; a desktop entry deve nomear diretamente o executável pretendido.

## Evidência

A decisão é implementada por `apps/klauncher/src/core/desktop.rs`,
`apps/klauncher/src/core/launch.rs` e
`apps/kbar/src/services/command.rs`.
