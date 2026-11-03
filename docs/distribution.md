# Distribuição: Steam, GOG e Steam Deck (preparação)

> **Atualização:** o usuário decidiu **não publicar** na Steam nem na GOG. O foco passou a ser usar o cliente da Steam como lançador local (ver `steam-library.md`). As seções abaixo servem como requisitos de Deck, caminhos por SO, assinatura e riscos, caso a decisão mude.

Status: **preparação**, nada publicado. Os repositórios continuam privados (regra 3) e
publicar numa loja é decisão posterior do usuário. Isto não é aconselhamento jurídico (ver
`LEGAL.md`). Itens "não confirmado" não foram verificados em fonte primária.

## 1. O que está confirmado
- **Deck Verified** ([Steamworks, compatibilidade do Deck](https://partner.steamgames.com/doc/steamdeck/compat)):
  - entrada: a configuração padrão dá acesso a todo o conteúdo; glifos acompanham o dispositivo
    ativo; texto digitável por controle;
  - tela: 1280x800 preferido (1280x720 aceito); menor texto >= 9 px (12 px recomendado);
  - launcher: precisa de navegação completa por controle; exigir navegação num launcher é
    "fortemente desencorajado";
  - desempenho: o padrão deve chegar a 30 fps a 800p;
  - não avisar que o hardware/software do Deck "não é suportado".
- **Proton x nativo:** com build Linux nativo ele é testado primeiro; sem ele, o build Windows roda
  sob Proton (mesma fonte).
- **GOG:** sem DRM e sem limites de instalação/ativação
  ([docs.gog.com/quality-assurance](https://docs.gog.com/quality-assurance/)); o SDK do Galaxy é
  opcional.
- **Windows Smart App Control** ([Microsoft](https://learn.microsoft.com/windows/apps/develop/smart-app-control/overview)):
  bloqueia por padrão código desconhecido e não assinado (eventos 3076/3077 no log
  CodeIntegrity). **Já vimos isso acontecer** neste projeto. Para distribuir é preciso **assinar
  o executável**; relatos de comunidade dizem que mesmo assinado, software novo pode ser
  bloqueado no início (fonte não oficial).
- **Precedentes:** OpenMW exige os dados do Morrowind do usuário; ScummVM está na Steam como
  add-on gratuito sem jogos incluídos. O Dolphin foi retirado da Steam sob pressão da Nintendo
  ([PC Gamer](https://www.pcgamer.com/valve-wouldnt-host-dolphin-on-steam-because-it-cant-sit-in-judgement-on-nintendos-fight-against-emulators));
  um port recompilado não é emulador, mas a Valve pode ver risco parecido (inferência).

## 2. Não confirmado (pesquisar antes de decidir)
- Fluxo "Add a Non-Steam Game" para .exe nativo, templates do Steam Input e artwork. (Os
  launchers do Akita documentam o passo manual em modo Desktop e forçam GE-Proton; sem automação.)
- Regras do Steamworks/Subscriber Agreement para software que exige arquivos do usuário.
- Política da GOG para ports que exigem arquivos do usuário e instaladores offline.
- Gamescope, limites de TDP/FPS, Flatpak/AppImage no Deck.
- Política de uso do nome/marca do jogo e de recompilações em Steam/GOG.

## 3. Decisões baratas para tomar já (priorizadas)
1. **Dados do usuário separados dos do programa.** Pasta de dados gerados (fora do git) e caminho
   do dump configurável; aceitar ISO/CUE/CHD.
2. **Módulo único de caminhos por SO:** portátil (ao lado do exe) ou perfil do usuário; subpastas
   separadas para logs, config, saves, cache de shader e dados gerados.
   - Windows: `%LOCALAPPDATA%` (e `%APPDATA%` para config) ou pasta portátil.
   - Linux/Deck: XDG (`~/.local/share`, `~/.config`, `~/.local/state`, `~/.cache`) — de memória,
     não reconfirmado na spec.
3. **Entrada via SDL3 gamepad** (cobre Steam Input e Deck); glifos trocam conforme o dispositivo;
   menus e launcher 100% navegáveis por controle.
4. **UI escalável** para 1280x800/1280x720; fonte mínima 12 px; opção de tamanho de texto.
5. **Validação do dump como uma tela do próprio jogo/launcher**, sem exigir teclado e mouse.
6. **Camada de renderer atrás de uma interface** (D3D11 -> D3D12 -> Vulkan; ver `gs-gpu-plan.md`).
   No Deck: D3D11 via Proton/DXVK é o caminho curto (inferência); Vulkan nativo depois.
7. **Funcionar offline e sem cliente de loja.** SDKs de Steam/Galaxy só como módulos opcionais,
   desligados por padrão.
8. **Assinatura de código no pipeline** (reservar a etapa) por causa do Smart App Control.
9. **Nome e metadados neutros** no executável (nome próprio do port; sem a marca do jogo);
   ISO, assets e hashes de conteúdo fora do repo (já é regra).
10. **Tudo configurável por flag e arquivo** (tela cheia, resolução, limite de FPS, caminho do
    dump, renderer), exigência prática para perfis do Gamescope/Steam.
11. **Perfil de execução para o Deck:** 1280x800, limite de FPS (30/40/60), renderer padrão
    seguro, sem diálogos que exijam mouse.
12. **Empacotamento:** zip portátil (Windows) + AppImage (Linux) no estilo Akita; instalador
    (MSI/NSIS/Inno) só se a loja exigir.

## 4. Próximas pesquisas
- Páginas do Steam Support/Steamworks sobre non-Steam e jogos que exigem dados do usuário.
- "Releasing Your Game on GOG" (Essentials Checklist, Offline Installers).
- Gamescope, Flatpak/AppImage e energia no Deck.
- Opções de certificado de assinatura (custo/prazo) e reputação no SmartScreen.
- Consultar um advogado antes de qualquer distribuição pública.
