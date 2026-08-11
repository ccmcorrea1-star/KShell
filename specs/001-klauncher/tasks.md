# Funcionalidade 001: tarefas do Klauncher

Estas são tarefas retrospectivas da baseline. Os itens concluídos registram
comportamentos que já existem; os itens em aberto exigem uma decisão de
produto antes da implementação.

## Baseline

- [x] Inventariar precedência de diretórios XDG e filtragem de desktop-files.
- [x] Registrar nomes localizados, tratamento de ícones e comportamento de
  field codes de `Exec` suportado.
- [x] Registrar ranking fuzzy e transições de seleção.
- [x] Registrar construção de comandos sem shell, fallback de terminal,
  diretório de trabalho e comportamento de sessão.
- [x] Mapear interações do launcher e restrições de layer-shell para evidências
  de aceite.
- [x] Confirmar que os artefatos CSS/KDL gerados do launcher vêm de
  `crates/theme`.

## Definições necessárias antes de trabalho futuro

- [ ] Decidir se a ativação de arquivos e URLs deve ser suportada.
- [ ] Decidir se ações de desktop ou outros comandos devem aparecer no launcher.
- [ ] Decidir se a geometria e o comportamento do launcher precisam de um
  contrato de configuração do usuário.
