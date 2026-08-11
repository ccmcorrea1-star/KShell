# Funcionalidade 004: tokens de tema compartilhados e geração

Status: baseline implementada, documentada retrospectivamente. Esta spec cobre
o renderer de tokens atual e seus consumidores versionados e opt-in do usuário.

## Classificação das evidências

- **Confirmado:** tokens canônicos, templates incorporados, outputs CSS/KDL/
  mockup gerados, `--write`/`--check`, detecção de Kitty/Alacritty/Foot/Cava/
  Fastfetch e testes de preservação estão implementados em `crates/theme/`,
  `tools/theme-gen/` e nos arquivos gerados.
- **Inferido:** `crates/theme` é a fonte visual única de verdade e outputs
  versionados são consumidores de build/runtime. Isso segue o renderer, os
  headers de geração e o documento de design.
- **TBD:** escrita transacional de configurações do usuário, um schema de tema
  voltado ao usuário e suporte a consumidores adicionais não estão definidos.

## Objetivo

Manter Klauncher, Kbar, fragments Niri, mockups e consumidores visuais locais
suportados alinhados a um único sistema de design, sem assumir a propriedade de
configurações do usuário que não sejam relacionadas ao tema.

## Requisitos e comportamento

### THEME-1 — Manter valores canônicos de forma centralizada

Cores, entradas de paleta semântica e ANSI, tipografia, spacing, radii,
bordas, geometria do launcher, geometria da barra e identificadores Niri
compartilhados DEVEM ser definidos por `crates/theme/src/tokens.rs` ou pelo
crate Niri compartilhado referenciado. Templates DEVEM usar esses valores em
vez de introduzir uma segunda fonte de verdade.

### THEME-2 — Renderizar consumidores versionados

O renderer DEVE resolver todos os tokens no CSS GTK do launcher, CSS GTK da
barra, fragments KDL Niri e CSS do mockup. `kshell-theme-gen --write` DEVE
atualizar os arquivos gerados versionados, e `--check` DEVE falhar quando
qualquer arquivo gerado estiver desatualizado ou a regra de opacity suportada
do Alacritty não for satisfeita.

Outputs gerados DEVEM manter seus headers de arquivo gerado e NÃO DEVEM ser
editados manualmente como uma fonte de design independente.

### THEME-3 — Atualizar apenas consumidores do usuário compatíveis

Quando `--write` for executado, Kitty, Alacritty, Foot, Cava e Fastfetch PODEM
ser atualizados somente quando o executável, a configuração do usuário e o
tema importado ou seção de cores ativa esperados estiverem presentes. A
presença apenas do executável ou de uma configuração órfã NÃO DEVE fazer com
que um novo arquivo de consumidor seja criado.

O renderer DEVE preservar fontes, shells, atalhos, módulos, logos, layout e
outras configurações não relacionadas. PODE definir opacity `1.0` em uma
janela existente do Alacritty, conforme exige a regra atual de ausência de
transparência.

### THEME-4 — Preservar a estrutura específica de cada consumidor

Atualizações de Cava DEVEM substituir somente a seção `[color]` ativa ou o
bloco de tema gerenciado existente. Atualizações de Fastfetch DEVEM substituir
somente valores de cor reconhecidos de logo e módulos. Arquivos de tema de
terminal DEVEM receber os valores de superfície e ANSI compartilhados sem
assumir a propriedade do comportamento do terminal.

### THEME-5 — Verificar o comportamento de rendering

O pacote de tema DEVE testar resolução de tokens, valores de backdrop/blur do
launcher/Niri, output completo da paleta de terminal, substituição de seção
Cava, normalização de opacity do Alacritty e substituição apenas de cores do
Fastfetch. O workspace deve conseguir verificar outputs gerados sem uma sessão
Wayland.

## Critérios de aceite

| ID | Critério | Evidência |
| --- | --- | --- |
| AC-1 | Todos os templates incorporados são resolvidos a partir do conjunto de tokens canônicos. | Teste unitário `templates_resolve_all_tokens` em `crates/theme/src/tokens.rs`. |
| AC-2 | Outputs CSS/KDL/mockup versionados são reproduzíveis e `--check` detecta divergências. | Implementação do generator e `cargo run -p kshell-theme-gen -- --check`. |
| AC-3 | Restrições visuais compartilhadas de launcher/bar/Niri e valores de blur permanecem alinhados. | Testes de tema e documento arquitetural do sistema de design. |
| AC-4 | Comportamento existente de Cava, Fastfetch, tema de terminal e opacity do Alacritty preserva configurações não relacionadas. | Testes colocados junto das transformações do tema. |
| AC-5 | Uma mudança de geração é seguida por validação de formato, testes, lint, build e outputs gerados. | Gates de validação do repositório e workflow de CI. |

## Fora do escopo desta baseline

- **TBD:** writes transacionais, backups ou rollback para arquivos de
  configuração do usuário.
- **TBD:** um schema formal de configuração de tema ou reload de tema em
  runtime.
- **TBD:** formatos de consumidores adicionais e políticas de descoberta
  automática.
