# Funcionalidade 004: tarefas do sistema de tema

Estas são tarefas retrospectivas da baseline para o renderer compartilhado
existente.

## Baseline

- [x] Inventariar tokens canônicos e cada template incorporado.
- [x] Mapear outputs gerados versionados para seus templates de origem.
- [x] Registrar regras de detecção e preservação de consumidores configurados.
- [x] Registrar transformações específicas de Cava, Fastfetch, terminal e
  Alacritty.
- [x] Confirmar que verificações de rendering e outputs gerados são
  automatizadas.
- [x] Migrar a documentação visual de design para `docs/architecture/`.

## Definições necessárias antes de trabalho futuro

- [ ] Decidir se writes de configuração do usuário precisam de backups ou
  transações.
- [ ] Decidir se temas precisam de schema de configuração voltado ao usuário
  ou reload em runtime.
- [ ] Decidir quais consumidores adicionais estão no escopo.
