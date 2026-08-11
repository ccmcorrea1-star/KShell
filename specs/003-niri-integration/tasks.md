# Funcionalidade 003: tarefas da integração Niri

Estas são tarefas retrospectivas da baseline para o contrato IPC e de
compositor atual.

## Baseline

- [x] Inventariar requests, events, campos de workspace e defaults Niri
  suportados.
- [x] Registrar transições de estado conscientes da saída e direcionamento por
  ID de workspace.
- [x] Registrar descoberta do socket, backoff de reconexão, reset de estado
  obsoleto e encerramento pelo receiver.
- [x] Centralizar e listar IDs de aplicações, namespaces, commands e binding.
- [x] Mapear consumidores KDL gerados e sua validação manual no Niri.

## Definições necessárias antes de trabalho futuro

- [ ] Decidir quais events/actions Niri adicionais, se houver, são necessários.
- [ ] Decidir se negociação de versão do protocolo é necessária.
- [ ] Decidir a política para múltiplas superfícies Kbar e outputs explícitos.
