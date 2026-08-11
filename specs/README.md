# Especificações de funcionalidades

Cada funcionalidade usa a mesma estrutura SDD de três arquivos:

- `spec.md` — comportamento, requisitos, classificação das evidências e critérios de aceite.
- `plan.md` — abordagem técnica, limites existentes, impacto e abordagem de validação.
- `tasks.md` — tarefas pequenas e executáveis, com o trabalho da baseline atual marcado como concluído e decisões não resolvidas mantidas explícitas.

As especificações iniciais são baselines retrospectivas de funcionalidades já
presentes no repositório. Elas não autorizam uma nova implementação. Um
requisito confirmado pelo código é marcado como **Confirmado**; um limite
inferido da organização atual é marcado como **Inferido**; um comportamento
que exige uma decisão de produto ou arquitetura é marcado como **TBD**.

| ID | Funcionalidade | Status |
| --- | --- | --- |
| [001](001-klauncher/) | Launcher de aplicações Klauncher | Baseline implementada |
| [002](002-kbar/) | Barra superior Kbar e serviços de status | Baseline implementada |
| [003](003-niri-integration/) | IPC Niri e integração com o compositor | Baseline implementada |
| [004](004-theme-system/) | Tokens de tema compartilhados e geração | Baseline implementada |

Os princípios globais estão na [constitution](../.specify/memory/constitution.md).
