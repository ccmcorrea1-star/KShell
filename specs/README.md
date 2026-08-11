# Especificações de funcionalidades

`spec.md` é o único arquivo obrigatório: registra comportamento, evidências,
critérios de aceite e limites da funcionalidade. As specs atuais são baselines
retrospectivas e não autorizam uma nova implementação.

Use arquivos auxiliares somente quando a mudança exigir:

- `plan.md` — abordagem para uma mudança ativa que atravesse limites, introduza
  uma decisão técnica ou tenha validação não trivial.
- `tasks.md` — tarefas verificáveis para trabalho ativo com várias etapas.

Não crie esses arquivos para completar um template ou repetir a spec. Requisitos
confirmados pelo código são **Confirmados**, limites inferidos são **Inferidos**
e decisões de produto/arquitetura ainda abertas são **TBD**.

| ID | Funcionalidade | Status |
| --- | --- | --- |
| [001](001-klauncher/) | Launcher de aplicações Klauncher | Baseline implementada |
| [002](002-kbar/) | Barra superior Kbar e serviços de status | Baseline implementada |
| [003](003-niri-integration/) | IPC Niri e integração com o compositor | Baseline implementada |
| [004](004-theme-system/) | Tokens de tema compartilhados e geração | Baseline implementada |

Os princípios globais estão na [constitution](../.specify/memory/constitution.md).
