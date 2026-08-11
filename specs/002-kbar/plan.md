# Funcionalidade 002: plano de implementação do Kbar

Status: plano de implementação atual para a baseline retrospectiva.

## Abordagem técnica

1. Mantenha ciclo de vida da barra, configuração layer-shell, bridges do
   contexto principal GTK e resolução de saída em `apps/kbar/src/app.rs`.
2. Mantenha composição e apresentação em `ui/`: workspaces, clock/calendário,
   módulos de status, popovers e estado de interação de volume.
3. Mantenha fontes externas atrás de `services/`. Use o helper de comandos
   existente e limitado para fontes baseadas em comandos e leituras diretas de
   sysfs para dados de bateria.
4. Mantenha threads de worker responsáveis por polling, ordem de ações e
   deduplicação de mudanças. Envie atualizações tipadas ao contexto GTK em vez
   de acessar GTK a partir das threads de worker.
5. Reutilize `crates/niri` para estado/foco de workspaces e `crates/theme` para
   geometria compartilhada e CSS gerado.

## Impacto arquitetural

Nenhuma arquitetura nova é necessária para a funcionalidade implementada. A
bridge existente entre worker e contexto principal e os limites de adapters de
serviço são suficientes. Uma nova fonte de status deve continuar sendo uma
atualização de serviço tipada e não deve fazer o GTK depender diretamente de
parsing de comandos ou do ciclo de vida de processos externos.

## Abordagem de validação

- Teste parsers, aritmética de datas, transições de estado, ordem de ações,
  agregação de workers e helpers determinísticos de interação de UI nos
  módulos colocados junto da implementação.
- Execute os gates do workspace e a verificação do tema gerado.
- Verifique manualmente posicionamento layer-shell, cliques de workspace Niri,
  popovers, controle de volume, transições de rede e visibilidade da bateria
  em uma sessão adequada.

## Hipóteses e definições em aberto

A separação worker/UI é **Inferida** a partir do código atual e é preservada
como limite de mudança. Orquestração de múltiplas saídas e política de
módulos/intervalos configurável são **TBD** e exigem decisões explícitas de
produto.
