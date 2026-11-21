// Custom CSS for DeadChaps' draft overlay (the draft link plus /obs) as an OBS browser source: laid out as the
// streamer's draft scene, with picks and bans as hero card art.

/** OBS's default browser-source CSS: keeps the overlay transparent. */
const OBS_DEFAULT_CSS = 'body { background-color: rgba(0, 0, 0, 0); margin: 0px auto; overflow: hidden; }';

/**
 * The draft scene at 1920×1080, measured on the streamer's background image (overlay/draft-overlay.css), which
 * draws the ban boxes and pick tables. The art isn't quite symmetric, so each side has its own numbers, from the
 * outer edge in.
 */
const DRAFT_SCENE = {
  width: 1920,
  height: 1080,
  /** Ban boxes: each one's left edge, inside the gold band. */
  ban: { left: [331, 442, 553], right: [1489, 1378, 1267], y: 604, width: 103, height: 131 },
  /** Pick cells in the bottom panel: each one's left edge and width. */
  pick: {
    left: [
      [22, 145],
      [167, 155],
      [322, 155],
      [477, 155],
      [632, 155],
      [787, 155],
    ],
    right: [
      [1753, 147],
      [1598, 155],
      [1443, 155],
      [1288, 155],
      [1133, 155],
      [978, 155],
    ],
    y: 847,
    height: 201,
  },
  /** DeadChaps' turn timer, in the gap between the two gold lines at the bottom. */
  timer: { x: 22, y: 1057, width: 1878, height: 4 },
} as const;

/**
 * Each hero's card art on the Deadlock API's asset server, keyed by the name DeadChaps puts in a pick's or ban's
 * `alt`: picks show the card (`{file}_card.webp`), bans its battered version (`{file}_card_critical.webp`). Both
 * are framed head and shoulders, so every hero fits a pick cell or ban box the same way. A hero missing here keeps
 * DeadChaps' own image.
 */
const HERO_CARDS: Record<string, string> = {
  Abrams: 'bull',
  Apollo: 'fencer',
  Baba: 'baba',
  Bebop: 'bebop',
  Billy: 'punkgoat',
  Calico: 'nano',
  Celeste: 'unicorn',
  'Deadman Danny': 'deadpack',
  Drifter: 'drifter',
  Dynamo: 'sumo',
  Graves: 'necro',
  'Grey Talon': 'archer',
  Haze: 'haze',
  Holliday: 'astro',
  Infernus: 'inferno',
  Ivy: 'tengu',
  Kelvin: 'kelvin',
  'Lady Geist': 'spectre',
  Lash: 'lash',
  McGinnis: 'engineer',
  Mina: 'vampirebat',
  Mirage: 'mirage',
  'Mo & Krill': 'digger',
  'Nurse Harrow': 'nurse',
  Paige: 'bookworm',
  Paradox: 'chrono',
  Pocket: 'synth',
  'Rat King': 'ratking',
  Rem: 'familiar',
  Seven: 'gigawatt',
  Shiv: 'shiv',
  Silver: 'werewolf',
  Sinclair: 'magician',
  Solomon: 'chessmaster',
  'The Doorman': 'doorman',
  Venator: 'priest',
  Victor: 'frank',
  Vindicta: 'hornet',
  Violet: 'artist',
  Viscous: 'viscous',
  Vyper: 'kali',
  Warden: 'warden',
  Wraith: 'wraith',
  Yamato: 'yamato',
};

const HERO_CARD_URL = 'https://assets-bucket.deadlock-api.com/assets-api-res/images/heroes';

/** Bans per side DeadChaps can show. */
const MAX_BANS = 3;

/** How a ban's or pick's art comes in: fading up while it settles into its box. */
const REVEAL = 'refresher-reveal 0.35s ease-out';

/** DeadChaps' overlay laid out as the scene: its own boxes and names hidden, bans and picks in the scene's boxes. */
function sceneCss(): string[] {
  const { width, height, ban, pick, timer } = DRAFT_SCENE;
  const rules = [
    `.draft-obs { position: relative !important; display: block !important; width: ${width}px !important; height: ${height}px !important; padding: 0 !important; }`,
    // Their rows and columns stop laying things out: every box below is placed on the scene.
    '.obs-heroes-section, .obs-team-column, .obs-team-header, .obs-team-content, .obs-picks-container, .obs-header-bans, .obs-bans-minimap-container { display: contents !important; }',
    '.obs-team-identity, .obs-bans-label { display: none !important; }',
    // The background draws the boxes, so DeadChaps' are see-through and only hold the art (and a ban's number).
    // A ban or pick is a new box in front of the empty ones, which each move a place along. DeadChaps' transition
    // would slide them over and its entrance pops the whole box in, so boxes stay put and only the art fades in.
    `.obs-minimap-ban { position: absolute !important; top: ${ban.y}px; width: ${ban.width}px !important; height: ${ban.height}px !important; margin: 0 !important; border: none !important; border-radius: 0 !important; background: none !important; box-shadow: none !important; display: flex !important; align-items: center; justify-content: center; overflow: hidden; transition: none !important; animation: none !important; }`,
    `.obs-minimap-ban.empty::after { font: 800 54px/1 Geist, 'Segoe UI', system-ui, sans-serif; color: rgba(255, 255, 255, 0.34); }`,
    // A ban is the hero's battered card, greyed out.
    `.obs-minimap-ban img { width: 100% !important; height: 100% !important; object-fit: cover !important; object-position: 50% 0%; filter: grayscale(0.9) saturate(0.4) brightness(0.8) contrast(1.05); animation: ${REVEAL}; }`,
    `.obs-hero-slot { position: absolute !important; top: ${pick.y}px; height: ${pick.height}px !important; margin: 0 !important; border-radius: 0 !important; background: none !important; overflow: hidden !important; transition: none !important; animation: none !important; }`,
    `.obs-hero-slot img { width: 100% !important; height: 100% !important; object-fit: cover !important; object-position: 50% 0%; animation: ${REVEAL}; }`,
    '@keyframes refresher-reveal { from { opacity: 0; transform: scale(1.08); } to { opacity: 1; transform: none; } }',
    `.obs-timer-bar { position: absolute !important; left: ${timer.x}px; top: ${timer.y}px; width: ${timer.width}px !important; height: ${timer.height}px !important; margin: 0 !important; }`,
    '.obs-timer-line.amber, .obs-timer-line.sapphire { background: #e4fbf3 !important; box-shadow: 0 0 8px rgba(140, 245, 210, 0.8), 0 0 20px rgba(52, 211, 166, 0.45) !important; }',
  ];
  for (let i = 1; i <= MAX_BANS; i++) {
    rules.push(`.obs-minimap-ban.empty:nth-child(${i})::after { content: "${i}"; }`);
  }
  // Column 1 is Hiddenking, on the left; column 2 is Archmother, mirrored on the right. Both fill in from the
  // outer edge.
  for (const [column, side] of [
    [1, 'left'],
    [2, 'right'],
  ] as const) {
    ban[side].forEach((left, i) => {
      rules.push(`.obs-team-column:nth-child(${column}) .obs-minimap-ban:nth-child(${i + 1}) { left: ${left}px; }`);
    });
    pick[side].forEach(([left, cellWidth], i) => {
      rules.push(`.obs-team-column:nth-child(${column}) .obs-hero-slot:nth-child(${i + 1}) { left: ${left}px; width: ${cellWidth}px !important; }`);
    });
  }
  return rules;
}

/** For the browser source's Custom CSS: OBS's default, the scene's layout, and picks and bans as card art. */
export const OBS_OVERLAY_CSS = [
  OBS_DEFAULT_CSS,
  '/* Refresher draft scene: bans and picks laid out in the scene. */',
  ...sceneCss(),
  '/* Picks as card art, bans as the battered card. */',
  ...Object.entries(HERO_CARDS).flatMap(([hero, file]) => [
    `.obs-hero-slot img[alt="${hero}"] { content: url("${HERO_CARD_URL}/${file}_card.webp"); }`,
    `.obs-minimap-ban img[alt="${hero}"] { content: url("${HERO_CARD_URL}/${file}_card_critical.webp"); }`,
  ]),
].join('\n');
