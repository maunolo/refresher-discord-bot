// Custom CSS for DeadChaps' draft overlay (the draft link plus /obs) as an OBS browser source: laid out as the
// streamer's draft scene, with picks and bans as hero card art.

/** OBS's default browser-source CSS: keeps the overlay transparent. */
const OBS_DEFAULT_CSS = 'body { background-color: rgba(0, 0, 0, 0); margin: 0px auto; overflow: hidden; }';

/**
 * The draft scene at 1920×1080, measured on the streamer's background image, which draws the ban boxes and pick
 * tables. Measured on the left side; the right side mirrors it.
 */
const DRAFT_SCENE = {
  width: 1920,
  height: 1080,
  /** Ban boxes: each one's left edge, from the outer edge in. */
  ban: { x: [390, 549, 709], y: 587, width: 143, height: 182 },
  /** A pick table's outer lines across, split evenly into a column per pick; its cells, below the row of names. */
  table: { x: 20, width: 922, cellTop: 848, cellBottom: 1048 },
  /** DeadChaps' turn timer, along the bottom. */
  timer: { x: 20, y: 1060, height: 4 },
} as const;

/** Where a box `width` wide that sits at `x` on the left side sits on `right`'s side. */
function mirrored(x: number, width: number, right: boolean): number {
  return right ? DRAFT_SCENE.width - x - width : x;
}

/**
 * Column `index` of the pick table starting at `x`, counted from its left: the pixels between its two lines, so
 * the art doesn't cover them. The lines are drawn about 1.5px wide, on fractional pixels.
 */
function tableColumn(x: number, index: number): { left: number; width: number } {
  const line = (i: number) => x + (i * DRAFT_SCENE.table.width) / MAX_PICKS;
  const left = Math.ceil(line(index) + 1);
  return { left, width: Math.floor(line(index + 1)) - left };
}

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

/** Bans and picks per side DeadChaps can show: 3 bans, 6 picks. */
const MAX_BANS = 3;
const MAX_PICKS = 6;

/** How a ban's or pick's art comes in: fading up while it settles into its box. */
const REVEAL = 'refresher-reveal 0.35s ease-out';

/** DeadChaps' overlay laid out as the scene: its own boxes and names hidden, bans and picks in the scene's boxes. */
function sceneCss(): string[] {
  const { width, height, ban, table, timer } = DRAFT_SCENE;
  const rules = [
    `.draft-obs { position: relative !important; display: block !important; width: ${width}px !important; height: ${height}px !important; padding: 0 !important; }`,
    // Their rows and columns stop laying things out: every box below is placed on the scene.
    '.obs-heroes-section, .obs-team-column, .obs-team-header, .obs-team-content, .obs-picks-container, .obs-header-bans, .obs-bans-minimap-container { display: contents !important; }',
    '.obs-team-identity, .obs-bans-label { display: none !important; }',
    // The background draws the boxes, so DeadChaps' are see-through and only hold the art (and a ban's number).
    // A ban or pick is a new box in front of the empty ones, which each move a place along. DeadChaps' transition
    // would slide them over and its entrance pops the whole box in, so boxes stay put and only the art fades in.
    `.obs-minimap-ban { position: absolute !important; top: ${ban.y}px; width: ${ban.width}px !important; height: ${ban.height}px !important; margin: 0 !important; border: none !important; border-radius: 0 !important; background: none !important; box-shadow: none !important; display: flex !important; align-items: center; justify-content: center; overflow: hidden; transition: none !important; animation: none !important; }`,
    `.obs-minimap-ban.empty::after { font: 800 64px/1 Geist, 'Segoe UI', system-ui, sans-serif; color: rgba(255, 255, 255, 0.34); }`,
    `.obs-minimap-ban img { object-fit: cover !important; object-position: 50% 0%; filter: brightness(0.85); animation: ${REVEAL}; }`,
    `.obs-hero-slot { position: absolute !important; top: ${table.cellTop}px; height: ${table.cellBottom - table.cellTop}px !important; margin: 0 !important; border-radius: 0 !important; background: none !important; overflow: hidden !important; transition: none !important; animation: none !important; }`,
    `.obs-hero-slot img { object-fit: cover !important; object-position: 50% 0%; animation: ${REVEAL}; }`,
    '@keyframes refresher-reveal { from { opacity: 0; transform: scale(1.08); } to { opacity: 1; transform: none; } }',
    `.obs-timer-bar { position: absolute !important; left: ${timer.x}px; top: ${timer.y}px; width: ${width - 2 * timer.x}px !important; height: ${timer.height}px !important; margin: 0 !important; }`,
    '.obs-timer-line.amber, .obs-timer-line.sapphire { background: #e4fbf3 !important; box-shadow: 0 0 8px rgba(140, 245, 210, 0.8), 0 0 20px rgba(52, 211, 166, 0.45) !important; }',
  ];
  for (let i = 1; i <= MAX_BANS; i++) {
    rules.push(`.obs-minimap-ban.empty:nth-child(${i})::after { content: "${i}"; }`);
  }
  // Column 1 is Hiddenking, on the left; column 2 is Archmother, mirrored on the right. Both fill in from the
  // outer edge.
  for (const column of [1, 2]) {
    const right = column === 2;
    ban.x.forEach((x, i) => {
      const left = mirrored(x, ban.width, right);
      rules.push(`.obs-team-column:nth-child(${column}) .obs-minimap-ban:nth-child(${i + 1}) { left: ${left}px; }`);
    });
    const tableX = mirrored(table.x, table.width, right);
    for (let i = 0; i < MAX_PICKS; i++) {
      const cell = tableColumn(tableX, right ? MAX_PICKS - 1 - i : i);
      rules.push(`.obs-team-column:nth-child(${column}) .obs-hero-slot:nth-child(${i + 1}) { left: ${cell.left}px; width: ${cell.width}px !important; }`);
    }
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
