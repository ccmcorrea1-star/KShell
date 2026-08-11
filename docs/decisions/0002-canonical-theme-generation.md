# ADR-0002: Manter uma fonte canônica de tema e gerar consumidores

- Status: Aceito
- Escopo: consumidores visuais do KShell e arquivos de configuração de
  usuário suportados

## Contexto

Klauncher, Kbar, fragments Niri, mockups de browser e configurações
suportadas de terminais ou visualizers precisam compartilhar os mesmos tokens
visuais. Manter cópias independentes permitiria que cores, geometria e
identificadores de compatibilidade divergissem.

## Decisão

Manter tokens canônicos e lógica de rendering em
`crates/theme/src/tokens.rs` e seus templates incorporados. Usar
`kshell-theme-gen --write` para atualizar consumidores versionados e `--check`
para detectar artefatos gerados desatualizados. Atualizar um consumidor do
usuário somente quando o executável, a configuração e o tema importado ou a
seção de cores ativa esperados estiverem presentes; preservar campos de
configuração não relacionados.

## Consequências

- Mudanças em tokens ou templates exigem regeneração e uma verificação dos
  outputs gerados.
- Arquivos CSS/KDL/mockup gerados são outputs, não fontes independentes de
  verdade.
- Configurações não relacionadas do usuário em terminal, Cava ou Fastfetch
  continuam sob responsabilidade dessas configurações.
- O generator atualmente escreve diretamente nos arquivos descobertos; writes
  transacionais e rollback não são definidos.

## Evidência

A decisão é implementada por `crates/theme/src/tokens.rs`,
`tools/theme-gen/src/main.rs`, pelos arquivos em `crates/theme/templates/` e
pelos headers de arquivos gerados.
