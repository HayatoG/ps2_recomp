# Integração com a biblioteca da Steam (uso local, sem loja)

Decisão do usuário: **não publicar na Steam nem na GOG.** O objetivo é abrir o port pelo cliente
da Steam como "jogo adicionado" (non-Steam), com nome, capa e controle configurado, inclusive no
Steam Deck. Isso vale só para a máquina do usuário; nada vai para loja. (`distribution.md`
continua útil só pelos requisitos de Deck, caminhos por SO e assinatura.)

Itens "não confirmado" não foram verificados em fonte primária.

## O que a Steam precisa
- **Atalho non-Steam**: entrada em `Steam/userdata/<UserID>/config/shortcuts.vdf` (VDF binário):
  nome, exe, pasta inicial, argumentos, ícone, tags. Fonte: [comunidade](https://steamcommunity.com/groups/homestream/discussions/0/355043117515861974)
  e [decky-romm-sync](https://danielcopper.github.io/decky-romm-sync/architecture/steam-non-steam-shortcuts/).
- **Arte** em `Steam/userdata/<UserID>/config/grid/`, nomeada pelo appid do atalho (mesma fonte):
  - `<appid>p.png` capa vertical (cartaz), `<appid>.png` grade larga, `<appid>_hero.png` banner,
    `<appid>_logo.png` logo, `<appid>_icon.png` ícone.
  - Tamanhos usuais (de memória, **não confirmado**): capa 600x900, larga 920x430, hero 1920x620.
- **appid do atalho**: as fontes divergem (CRC32 de "exe entre aspas + nome", ou só do nome, com
  `| 0x80000000`). **Não calcular às cegas**: criar o atalho, ler o appid que a Steam gravou, e só
  então nomear a arte. O decky-romm-sync também lê o valor em vez de calcular.
- **Controle**: a Steam Input aplica template a jogos non-Steam; o port usa SDL3 gamepad, então o
  layout padrão do Deck funciona (**não confirmado** na prática; testar).
- Fechar a Steam antes de editar `shortcuts.vdf` (ela sobrescreve ao sair; **não confirmado**,
  prática comum de ferramentas como Steam ROM Manager).

## Dois caminhos
1. **Manual** (documentado no README): Steam > Adicionar um jogo non-Steam > escolher o exe do
   launcher; renomear; definir ícone/capa nas propriedades.
2. **Automático** no launcher: comando `steam-add` (nome, exe, pasta, argumentos, arte). Usa uma
   biblioteca de VDF, faz **backup** de `shortcuts.vdf` antes, é idempotente (não duplica), pede
   para fechar a Steam e registra tudo no log. Alternativa: apontar o usuário para o Steam ROM
   Manager (**não confirmado** que ele sirva para exe nativo sem catálogo).

O atalho deve apontar para o **launcher** (não direto para o runtime), assim a validação do dump e
a escolha de renderer/perfil rodam sempre; o launcher precisa funcionar sem janela (`--no-window`)
e com gamepad, por causa do Deck.

## Arte: sai do dump do próprio usuário
Decisão do usuário: a arte do atalho pode vir **das artes disponíveis no próprio dump**, e o port é
de uso pessoal (só ele usa). Isto casa com as regras 2, 4 e 5: o launcher gera a arte **a partir do
dump que o usuário fornece**, grava só em pasta local ignorada pelo git (`<dados>/artwork/`) e
instala no `grid/` da Steam. Continua valendo: **nunca commitar** capas, logos, texturas ou
screenshots do jogo, e não distribuir o resultado a terceiros (se um dia o port for compartilhado,
cada pessoa gera a arte do seu próprio dump).

Fontes possíveis, em ordem de preferência (**a verificar o que o disco realmente tem**; um disco de
PS2 normalmente não traz "box art", e nem o ícone 3D do memory card está no disco):
1. Texturas do próprio jogo (logo/título/imagens de menu em `IMAGE.BIN`, formato zim, já lido por
   `tools/zim.py`): extrair e converter para PNG localmente.
2. Captura do frame da tela de título gerada pelo nosso runtime (`--shot-dir` com o script de pad
   da tela de título; já temos esse caminho funcionando).
3. Imagem fornecida pelo usuário em `<dados>/artwork/` (substitui as automáticas).

Composição (landscape de 4:3 ou 16:9 para os formatos da Steam): grade larga e hero cabem direto;
a capa vertical (600x900, **não confirmado**) precisa de enquadramento (corte central ou fundo
desfocado); logo e ícone só se houver arte isolada no disco. O comando `artwork` do launcher gera
os PNGs nos tamanhos da Steam, e o `steam-add` os instala. Nome do atalho: o título do jogo
(já que é uso pessoal) ou outro à escolha do usuário.

## Steam Deck
- Modo Desktop: adicionar o launcher como non-Steam (ou rodar o `steam-add` lá); modo Jogo: abre
  pela biblioteca. Execução do `.exe` sob Proton (forçar um Proton nas propriedades) até haver
  build Linux nativo. Resolução 1280x800, limite de FPS e renderer padrão pelo perfil "Deck".
- Dados e logs fora de pastas somente leitura; no Linux seguir XDG (`distribution.md`).

## Etapas
1. README com o passo manual + perfil de execução do Deck no `game.toml`/`launcher.cfg`.
2. `launcher.cfg` com `profile = desktop|deck` e flags equivalentes.
3. `artwork` (extrai/gera PNGs do dump, local) e `steam-add` (backup, idempotente, log; instala a arte em `grid/`). Antes: levantar quais imagens o disco tem.
4. Testar no PC (Windows) e depois num Deck real (nunca testado até aqui).
