# Estrutura de testes

A suíte de testes atual fica intencionalmente junto dos módulos de produção,
em seções `#[cfg(test)]`. Isso mantém helpers privados de parsing e estado
próximos do comportamento que verificam e evita mover testes estáveis apenas
para criar uma segunda hierarquia.

A cobertura atual inclui:

- `apps/klauncher/src/core/` — parsing de arquivos desktop, expansão de
  `Exec`, ranking fuzzy e construção de comandos de launch.
- `apps/klauncher/src/ui/` — transições de seleção, texto do estado vazio e
  cálculos de geometria.

O diretório raiz `tests/` é reservado para testes de integração que precisem
exercitar mais de um pacote. Atualmente ele contém apenas documentação; ainda
não há comportamento entre pacotes definido. GTK/layer-shell e o Niri ou
serviço do sistema em execução ainda exigem validação manual em uma sessão
adequada.
