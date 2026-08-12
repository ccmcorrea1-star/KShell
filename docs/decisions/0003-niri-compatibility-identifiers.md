# ADR-0003: Separar IDs GTK dos identificadores Niri

- Status: Aceito
- Escopo: IDs GTK, consumidores IPC Niri, superfícies layer-shell e fragments gerados

## Contexto

O projeto foi renomeado para KShell e os IDs históricos das aplicações GTK não
usavam o namespace do projeto. A configuração Niri local, por outro lado,
depende dos namespaces layer-shell, dos commands de inicialização e do binding
do launcher. Os fragments gerados e as superfícies GTK precisam continuar
alinhados nesses valores sem confundir a identidade GTK com o contrato do
compositor.

Os fragments Niri não referenciam os IDs GTK. A migração dos IDs, portanto,
não exige uma alteração nos fragments nem nas regras existentes que casam
namespaces layer-shell.

## Decisão

Usar estes IDs GTK no namespace do KShell, definidos diretamente nos módulos
das aplicações:

- launcher: `io.github.ccmcorrea1.kshell.Launcher`;
- bar: `io.github.ccmcorrea1.kshell.Bar`.

Manter os identificadores de integração Niri centralizados em
`crates/niri/src/lib.rs` e usá-los nos consumidores nativos e templates de
tema. Preservar os namespaces layer-shell `my-shell-launcher` e
`my-shell-bar`, o command de autostart `kbar`, o command do launcher
`klauncher` e o binding `Mod+Space` até que uma migração coordenada do
compositor seja especificada.

## Consequências

- A identidade GTK passa a ser consistente com o repositório KShell.
- A configuração Niri existente continua direcionando os binários e as regras
  layer-shell atuais.
- Uma mudança em namespaces, commands ou binding continua sendo uma mudança de
  compatibilidade e exige regeneração dos fragments e atualização da
  configuração do compositor.
- Os IDs GTK não precisam ser replicados nos templates KDL porque o Niri não os
  consome.

## Evidência

Os IDs GTK são definidos em `apps/klauncher/src/ui/gtk.rs` e
`apps/kbar/src/app.rs`. Os identificadores Niri compartilhados são definidos em
`crates/niri/src/lib.rs` e renderizados em `contrib/niri/*.kdl` pelo theme
generator.
