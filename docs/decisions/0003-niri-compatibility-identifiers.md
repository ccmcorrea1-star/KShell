# ADR-0003: Preservar identificadores Niri de compatibilidade

- Status: Aceito
- Escopo: consumidores IPC Niri, superfícies layer-shell e fragments gerados

## Contexto

A configuração Niri local existente referencia os nomes atuais de commands do
KShell, IDs das aplicações, namespaces layer-shell e shortcut do launcher. Os
fragments gerados e as superfícies GTK nativas precisam concordar nesses
valores.

## Decisão

Manter identificadores Niri compartilhados centralizados em
`crates/niri/src/lib.rs` e usá-los nos consumidores nativos e templates de
tema. Manter os IDs de aplicações GTK existentes em seus módulos atuais
(`com.klaucher.Bar` e `com.klaucher.Launcher`) até que uma migração coordenada
seja especificada explicitamente. Preservar o binding `Mod+Space` do launcher,
o command de autostart `kbar` e os namespaces `my-shell-*`.

## Consequências

- Uma renomeação é uma mudança de compatibilidade, não uma edição local de
  string.
- Arquivos Niri gerados devem ser regenerados depois de uma mudança de
  identificador aprovada.
- A configuração existente do usuário continua direcionando os binários e as
  regras layer-shell atuais.

## Evidência

Os identificadores compartilhados são definidos em `crates/niri/src/lib.rs`, os
IDs das aplicações são definidos nos módulos GTK das aplicações e os valores
Niri são renderizados em `contrib/niri/*.kdl` pelo theme generator.
