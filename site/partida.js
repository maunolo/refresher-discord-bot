// Resultados das partidas (placar do jogo + API pública do Deadlock). Ícones em assets/partida/.
// Partida 1: real (print do jogo). Partida 2: itens e números de uma partida real, mas com NOMES FICTÍCIOS.
//
// MVP: escolhido pelo ADM (não é o MVP do jogo). Em cada partida, troque "mvp": null pelo nome
// do jogador exatamente como está em "nome", por exemplo: "mvp": "VioletaS2". Com null aparece "A definir".
window.PARTIDAS = {
 "partidas": [
  {
   "mvp": null,
   "id": 139086680,
   "inicio": 1794919042,
   "duracao": 2026,
   "vencedor": 0,
   "times": [
    {
     "nome": "Hidden King",
     "classe": "king",
     "almas": "270k",
     "jogadores": [
      {
       "nome": "VioletaS2",
       "heroi": "Paige",
       "img": "paige",
       "almas": "40k",
       "k": 0,
       "d": 6,
       "a": 24,
       "dano": "19k",
       "obj": "979",
       "cura": "25k",
       "itens": [
        "opening-rounds",
        "trophy-collector",
        "arcane-surge",
        "knockdown",
        "silence-wave",
        "healing-nova",
        "divine-barrier",
        "stamina-mastery",
        "spirit-resilience",
        "transcendent-cooldown",
        "greater-expansion"
       ]
      },
      {
       "nome": "nascido e criado no brejo",
       "heroi": "Wraith",
       "img": "wraith",
       "almas": "60k",
       "k": 22,
       "d": 3,
       "a": 8,
       "dano": "55k",
       "obj": "13k",
       "cura": "18k",
       "itens": [
        "swift-striker",
        "extra-health",
        "surge-of-power",
        "enduring-speed",
        "dispel-magic",
        "mercurial-magnum",
        "capacitor",
        "cheat-death",
        "spirit-rend",
        "spiritual-overflow",
        "ricochet",
        "boundless-spirit"
       ]
      },
      {
       "nome": "Zibra234",
       "heroi": "Shiv",
       "img": "shiv",
       "almas": "37k",
       "k": 4,
       "d": 8,
       "a": 4,
       "dano": "22k",
       "obj": "2.9k",
       "cura": "6.3k",
       "itens": [
        "restorative-shot",
        "extra-charge",
        "monster-rounds",
        "radiant-regeneration",
        "healbane",
        "torment-pulse",
        "escalating-exposure",
        "superior-cooldown",
        "mystic-reverb",
        "unstoppable"
       ]
      },
      {
       "nome": "gOldeN gOd",
       "heroi": "Victor",
       "img": "victor",
       "almas": "40k",
       "k": 3,
       "d": 4,
       "a": 6,
       "dano": "17k",
       "obj": "9.2k",
       "cura": "9.3k",
       "itens": [
        "improved-spirit",
        "healing-booster",
        "cultist-sacrifice",
        "surge-of-power",
        "greater-expansion",
        "spirit-lifesteal",
        "rapid-recharge",
        "torment-pulse",
        "radiant-regeneration",
        "juggernaut",
        "escalating-exposure"
       ]
      },
      {
       "nome": "Fortnite Battlepass",
       "heroi": "Apollo",
       "img": "apollo",
       "almas": "57k",
       "k": 11,
       "d": 6,
       "a": 6,
       "dano": "51k",
       "obj": "25k",
       "cura": "15k",
       "itens": [
        "duration-extender",
        "spirit-snatch",
        "counterspell",
        "healing-booster",
        "hunter-s-aura",
        "tankbuster",
        "spirit-resilience",
        "boundless-spirit",
        "greater-expansion",
        "crushing-fists",
        "escalating-exposure",
        "torment-pulse"
       ]
      },
      {
       "nome": "Sir Cunferência",
       "heroi": "Sinclair",
       "img": "sinclair",
       "almas": "36k",
       "k": 3,
       "d": 10,
       "a": 9,
       "dano": "22k",
       "obj": "1.1k",
       "cura": "2.5k",
       "itens": [
        "high-velocity-rounds",
        "improved-spirit",
        "rapid-recharge",
        "superior-duration",
        "greater-expansion",
        "disarming-hex",
        "spirit-shielding",
        "vortex-web",
        "plated-armor",
        "knockdown"
       ]
      }
     ]
    },
    {
     "nome": "Archmother",
     "classe": "arch",
     "almas": "216k",
     "jogadores": [
      {
       "nome": "yakui",
       "heroi": "Mina",
       "img": "mina",
       "almas": "29k",
       "k": 0,
       "d": 9,
       "a": 5,
       "dano": "13k",
       "obj": "3.7k",
       "cura": "3.6k",
       "itens": [
        "extra-health",
        "quicksilver-reload",
        "mystic-expansion",
        "improved-spirit",
        "spirit-shielding",
        "tankbuster",
        "spirit-resilience",
        "stamina-mastery",
        "swift-striker",
        "toxic-bullets",
        "superior-cooldown"
       ]
      },
      {
       "nome": "lowea",
       "heroi": "Pocket",
       "img": "pocket",
       "almas": "35k",
       "k": 4,
       "d": 9,
       "a": 8,
       "dano": "28k",
       "obj": "4.1k",
       "cura": "2.3k",
       "itens": [
        "battle-vest",
        "mystic-vulnerability",
        "cold-front",
        "swift-striker",
        "tankbuster",
        "healbane",
        "superior-cooldown",
        "majestic-leap",
        "greater-expansion",
        "spirit-burn",
        "debuff-reducer"
       ]
      },
      {
       "nome": "[DATA LOST]",
       "heroi": "Abrams",
       "img": "abrams",
       "almas": "36k",
       "k": 3,
       "d": 8,
       "a": 4,
       "dano": "11k",
       "obj": "147",
       "cura": "13k",
       "itens": [
        "lifestrike",
        "point-blank",
        "spirit-resilience",
        "spirit-snatch",
        "unstoppable",
        "hunter-s-aura",
        "crushing-fists"
       ]
      },
      {
       "nome": "senegal",
       "heroi": "Celeste",
       "img": "celeste",
       "almas": "39k",
       "k": 7,
       "d": 4,
       "a": 8,
       "dano": "30k",
       "obj": "5.8k",
       "cura": "14k",
       "itens": [
        "mystic-burst",
        "swift-striker",
        "torment-pulse",
        "compress-cooldown",
        "opening-rounds",
        "healbane",
        "greater-expansion",
        "spirit-rend",
        "escalating-exposure",
        "plated-armor",
        "leech"
       ]
      },
      {
       "nome": "Skadush",
       "heroi": "Venator",
       "img": "venator",
       "almas": "39k",
       "k": 6,
       "d": 6,
       "a": 11,
       "dano": "38k",
       "obj": "4.9k",
       "cura": "9.2k",
       "itens": [
        "battle-vest",
        "cultist-sacrifice",
        "swift-striker",
        "enduring-speed",
        "weighted-shots",
        "fleetfoot",
        "escalating-resilience",
        "spirit-resilience",
        "silencer",
        "debuff-reducer",
        "vampiric-burst"
       ]
      },
      {
       "nome": "o",
       "heroi": "Viscous",
       "img": "viscous",
       "almas": "38k",
       "k": 17,
       "d": 7,
       "a": 6,
       "dano": "54k",
       "obj": "2.4k",
       "cura": "12k",
       "itens": [
        "express-shot",
        "mystic-shot",
        "spirit-shielding",
        "arcane-surge",
        "tankbuster",
        "warp-stone",
        "superior-cooldown",
        "greater-expansion",
        "torment-pulse",
        "superior-duration",
        "diviner-s-kevlar"
       ]
      }
     ]
    }
   ]
  },
  {
   "mvp": null,
   "id": null,
   "inicio": 1794919042,
   "duracao": 2623,
   "vencedor": 1,
   "times": [
    {
     "nome": "Hidden King",
     "classe": "king",
     "almas": "273k",
     "jogadores": [
      {
       "nome": "Coruja Noturna",
       "heroi": "Haze",
       "img": "haze",
       "almas": "40k",
       "k": 8,
       "d": 15,
       "a": 11,
       "dano": "46k",
       "obj": "2.4k",
       "cura": "10k",
       "itens": [
        "stalker",
        "swift-striker",
        "kinetic-dash",
        "titanic-magazine",
        "fury-trance",
        "fleetfoot",
        "burst-fire",
        "ricochet",
        "spirit-resilience",
        "rebuttal",
        "lucky-shot",
        "counterspell"
       ]
      },
      {
       "nome": "Mestre do Farm",
       "heroi": "Lady Geist",
       "img": "lady-geist",
       "almas": "58k",
       "k": 15,
       "d": 8,
       "a": 18,
       "dano": "86k",
       "obj": "4.7k",
       "cura": "50k",
       "itens": [
        "battle-vest",
        "healbane",
        "tankbuster",
        "spirit-resilience",
        "rapid-recharge",
        "escalating-exposure",
        "boundless-spirit",
        "spirit-burn",
        "transcendent-cooldown",
        "refresher",
        "radiant-regeneration",
        "healing-booster"
       ]
      },
      {
       "nome": "ZeroPing",
       "heroi": "Warden",
       "img": "warden",
       "almas": "46k",
       "k": 12,
       "d": 9,
       "a": 17,
       "dano": "36k",
       "obj": "4k",
       "cura": "2.9k",
       "itens": [
        "mystic-expansion",
        "enchanter-s-emblem",
        "compress-cooldown",
        "titanic-magazine",
        "fleetfoot",
        "spirit-resilience",
        "spellslinger",
        "surge-of-power",
        "mercurial-magnum",
        "berserker",
        "heroic-aura",
        "silencer"
       ]
      },
      {
       "nome": "Gelo no Copo",
       "heroi": "Wraith",
       "img": "wraith",
       "almas": "39k",
       "k": 4,
       "d": 8,
       "a": 12,
       "dano": "22k",
       "obj": "8.8k",
       "cura": "5.3k",
       "itens": [
        "monster-rounds",
        "restorative-shot",
        "quicksilver-reload",
        "surge-of-power",
        "swift-striker",
        "boundless-spirit",
        "ricochet",
        "leech",
        "escalating-resilience",
        "capacitor"
       ]
      },
      {
       "nome": "RushB_Raiz",
       "heroi": "Vyper",
       "img": "vyper",
       "almas": "44k",
       "k": 7,
       "d": 13,
       "a": 10,
       "dano": "36k",
       "obj": "4.7k",
       "cura": "13k",
       "itens": [
        "bullet-lifesteal",
        "improved-spirit",
        "tesla-bullets",
        "point-blank",
        "titanic-magazine",
        "toxic-bullets",
        "burst-fire",
        "lucky-shot",
        "fortitude",
        "glass-cannon",
        "radiant-regeneration",
        "siphon-bullets"
       ]
      },
      {
       "nome": "Tio do Suporte",
       "heroi": "Mo & Krill",
       "img": "mo-krill",
       "almas": "45k",
       "k": 3,
       "d": 7,
       "a": 15,
       "dano": "33k",
       "obj": "350",
       "cura": "9.9k",
       "itens": [
        "healing-booster",
        "improved-spirit",
        "trophy-collector",
        "torment-pulse",
        "return-fire",
        "superior-duration",
        "transcendent-cooldown",
        "diviner-s-kevlar",
        "escalating-exposure",
        "quicksilver-reload",
        "surge-of-power",
        "tankbuster"
       ]
      }
     ]
    },
    {
     "nome": "Archmother",
     "classe": "arch",
     "almas": "288k",
     "jogadores": [
      {
       "nome": "ShadowViper",
       "heroi": "Billy",
       "img": "billy",
       "almas": "46k",
       "k": 12,
       "d": 14,
       "a": 18,
       "dano": "62k",
       "obj": "11k",
       "cura": "12k",
       "itens": [
        "healing-rite",
        "berserker",
        "rapid-recharge",
        "diviner-s-kevlar",
        "crushing-fists",
        "lifestrike",
        "spirit-snatch",
        "enduring-speed",
        "superior-duration",
        "superior-cooldown",
        "colossus"
       ]
      },
      {
       "nome": "LunaCrit",
       "heroi": "McGinnis",
       "img": "mcginnis",
       "almas": "48k",
       "k": 4,
       "d": 2,
       "a": 15,
       "dano": "30k",
       "obj": "12k",
       "cura": "13k",
       "itens": [
        "monster-rounds",
        "mystic-slow",
        "spirit-rend",
        "spirit-burn",
        "boundless-spirit",
        "transcendent-cooldown",
        "healing-tempo",
        "rapid-recharge",
        "escalating-exposure"
       ]
      },
      {
       "nome": "Bruxo do Lag",
       "heroi": "Infernus",
       "img": "infernus",
       "almas": "54k",
       "k": 13,
       "d": 8,
       "a": 18,
       "dano": "74k",
       "obj": "8.7k",
       "cura": "22k",
       "itens": [
        "rapid-recharge",
        "enchanter-s-emblem",
        "infuser",
        "escalating-exposure",
        "duration-extender",
        "spirit-burn",
        "superior-cooldown",
        "spirit-resilience",
        "greater-expansion",
        "improved-spirit",
        "plated-armor",
        "healing-booster"
       ]
      },
      {
       "nome": "Kaiser77",
       "heroi": "Calico",
       "img": "calico",
       "almas": "46k",
       "k": 11,
       "d": 8,
       "a": 23,
       "dano": "35k",
       "obj": "3k",
       "cura": "24k",
       "itens": [
        "enchanter-s-emblem",
        "opening-rounds",
        "lifestrike",
        "crushing-fists",
        "torment-pulse",
        "radiant-regeneration",
        "spirit-snatch",
        "compress-cooldown",
        "rescue-beam",
        "stamina-mastery",
        "leech",
        "refresher"
       ]
      },
      {
       "nome": "Pato Sniper",
       "heroi": "Mina",
       "img": "mina",
       "almas": "44k",
       "k": 15,
       "d": 13,
       "a": 20,
       "dano": "56k",
       "obj": "3.6k",
       "cura": "5.9k",
       "itens": [
        "enchanter-s-emblem",
        "kinetic-dash",
        "restorative-locket",
        "enduring-speed",
        "tankbuster",
        "burst-fire",
        "mercurial-magnum",
        "escalating-exposure",
        "escalating-resilience",
        "boundless-spirit"
       ]
      },
      {
       "nome": "NoScopeNina",
       "heroi": "Dynamo",
       "img": "dynamo",
       "almas": "50k",
       "k": 4,
       "d": 4,
       "a": 15,
       "dano": "34k",
       "obj": "2.7k",
       "cura": "4.1k",
       "itens": [
        "greater-expansion",
        "superior-duration",
        "rapid-recharge",
        "tankbuster",
        "healing-tempo",
        "transcendent-cooldown",
        "diviner-s-kevlar",
        "refresher"
       ]
      }
     ]
    }
   ]
  }
 ],
 "itens": {
  "arcane-surge": {
   "nome": "Arcane Surge",
   "tipo": "spirit",
   "tier": 2,
   "ativo": false
  },
  "battle-vest": {
   "nome": "Battle Vest",
   "tipo": "vitality",
   "tier": 2,
   "ativo": false
  },
  "berserker": {
   "nome": "Berserker",
   "tipo": "weapon",
   "tier": 3,
   "ativo": false
  },
  "boundless-spirit": {
   "nome": "Boundless Spirit",
   "tipo": "spirit",
   "tier": 4,
   "ativo": false
  },
  "bullet-lifesteal": {
   "nome": "Bullet Lifesteal",
   "tipo": "vitality",
   "tier": 2,
   "ativo": false
  },
  "burst-fire": {
   "nome": "Burst Fire",
   "tipo": "weapon",
   "tier": 3,
   "ativo": false
  },
  "capacitor": {
   "nome": "Capacitor",
   "tipo": "weapon",
   "tier": 4,
   "ativo": true
  },
  "cheat-death": {
   "nome": "Cheat Death",
   "tipo": "vitality",
   "tier": 4,
   "ativo": false
  },
  "cold-front": {
   "nome": "Cold Front",
   "tipo": "spirit",
   "tier": 2,
   "ativo": true
  },
  "colossus": {
   "nome": "Colossus",
   "tipo": "vitality",
   "tier": 4,
   "ativo": true
  },
  "compress-cooldown": {
   "nome": "Compress Cooldown",
   "tipo": "spirit",
   "tier": 2,
   "ativo": false
  },
  "counterspell": {
   "nome": "Counterspell",
   "tipo": "vitality",
   "tier": 3,
   "ativo": false
  },
  "crushing-fists": {
   "nome": "Crushing Fists",
   "tipo": "weapon",
   "tier": 4,
   "ativo": false
  },
  "cultist-sacrifice": {
   "nome": "Cultist Sacrifice",
   "tipo": "weapon",
   "tier": 3,
   "ativo": true
  },
  "debuff-reducer": {
   "nome": "Debuff Reducer",
   "tipo": "vitality",
   "tier": 2,
   "ativo": false
  },
  "disarming-hex": {
   "nome": "Disarming Hex",
   "tipo": "spirit",
   "tier": 3,
   "ativo": true
  },
  "dispel-magic": {
   "nome": "Dispel Magic",
   "tipo": "vitality",
   "tier": 3,
   "ativo": true
  },
  "divine-barrier": {
   "nome": "Divine Barrier",
   "tipo": "vitality",
   "tier": 4,
   "ativo": true
  },
  "diviner-s-kevlar": {
   "nome": "Diviner's Kevlar",
   "tipo": "vitality",
   "tier": 4,
   "ativo": false
  },
  "duration-extender": {
   "nome": "Duration Extender",
   "tipo": "spirit",
   "tier": 2,
   "ativo": false
  },
  "enchanter-s-emblem": {
   "nome": "Enchanter's Emblem",
   "tipo": "vitality",
   "tier": 2,
   "ativo": false
  },
  "enduring-speed": {
   "nome": "Enduring Speed",
   "tipo": "vitality",
   "tier": 2,
   "ativo": false
  },
  "escalating-exposure": {
   "nome": "Escalating Exposure",
   "tipo": "spirit",
   "tier": 4,
   "ativo": false
  },
  "escalating-resilience": {
   "nome": "Escalating Resilience",
   "tipo": "weapon",
   "tier": 3,
   "ativo": false
  },
  "express-shot": {
   "nome": "Express Shot",
   "tipo": "weapon",
   "tier": 3,
   "ativo": false
  },
  "extra-charge": {
   "nome": "Extra Charge",
   "tipo": "spirit",
   "tier": 1,
   "ativo": false
  },
  "extra-health": {
   "nome": "Extra Health",
   "tipo": "vitality",
   "tier": 1,
   "ativo": false
  },
  "fleetfoot": {
   "nome": "Fleetfoot",
   "tipo": "weapon",
   "tier": 2,
   "ativo": true
  },
  "fortitude": {
   "nome": "Fortitude",
   "tipo": "vitality",
   "tier": 3,
   "ativo": false
  },
  "fury-trance": {
   "nome": "Fury Trance",
   "tipo": "vitality",
   "tier": 3,
   "ativo": true
  },
  "glass-cannon": {
   "nome": "Glass Cannon",
   "tipo": "weapon",
   "tier": 4,
   "ativo": false
  },
  "greater-expansion": {
   "nome": "Greater Expansion",
   "tipo": "spirit",
   "tier": 3,
   "ativo": false
  },
  "healbane": {
   "nome": "Healbane",
   "tipo": "vitality",
   "tier": 2,
   "ativo": false
  },
  "healing-booster": {
   "nome": "Healing Booster",
   "tipo": "vitality",
   "tier": 2,
   "ativo": false
  },
  "healing-nova": {
   "nome": "Healing Nova",
   "tipo": "vitality",
   "tier": 3,
   "ativo": true
  },
  "healing-rite": {
   "nome": "Healing Rite",
   "tipo": "vitality",
   "tier": 1,
   "ativo": true
  },
  "healing-tempo": {
   "nome": "Healing Tempo",
   "tipo": "vitality",
   "tier": 4,
   "ativo": false
  },
  "heroic-aura": {
   "nome": "Heroic Aura",
   "tipo": "weapon",
   "tier": 3,
   "ativo": true
  },
  "high-velocity-rounds": {
   "nome": "High-Velocity Rounds",
   "tipo": "weapon",
   "tier": 1,
   "ativo": false
  },
  "hunter-s-aura": {
   "nome": "Hunter's Aura",
   "tipo": "weapon",
   "tier": 3,
   "ativo": false
  },
  "improved-spirit": {
   "nome": "Improved Spirit",
   "tipo": "spirit",
   "tier": 2,
   "ativo": false
  },
  "infuser": {
   "nome": "Infuser",
   "tipo": "vitality",
   "tier": 4,
   "ativo": true
  },
  "juggernaut": {
   "nome": "Juggernaut",
   "tipo": "vitality",
   "tier": 4,
   "ativo": false
  },
  "kinetic-dash": {
   "nome": "Kinetic Dash",
   "tipo": "weapon",
   "tier": 2,
   "ativo": false
  },
  "knockdown": {
   "nome": "Knockdown",
   "tipo": "spirit",
   "tier": 3,
   "ativo": true
  },
  "leech": {
   "nome": "Leech",
   "tipo": "vitality",
   "tier": 4,
   "ativo": false
  },
  "lifestrike": {
   "nome": "Lifestrike",
   "tipo": "vitality",
   "tier": 3,
   "ativo": false
  },
  "lucky-shot": {
   "nome": "Lucky Shot",
   "tipo": "weapon",
   "tier": 4,
   "ativo": false
  },
  "majestic-leap": {
   "nome": "Majestic Leap",
   "tipo": "vitality",
   "tier": 3,
   "ativo": true
  },
  "mercurial-magnum": {
   "nome": "Mercurial Magnum",
   "tipo": "spirit",
   "tier": 4,
   "ativo": false
  },
  "monster-rounds": {
   "nome": "Monster Rounds",
   "tipo": "weapon",
   "tier": 1,
   "ativo": false
  },
  "mystic-burst": {
   "nome": "Mystic Burst",
   "tipo": "spirit",
   "tier": 1,
   "ativo": false
  },
  "mystic-expansion": {
   "nome": "Mystic Expansion",
   "tipo": "spirit",
   "tier": 1,
   "ativo": false
  },
  "mystic-reverb": {
   "nome": "Mystic Reverb",
   "tipo": "spirit",
   "tier": 4,
   "ativo": false
  },
  "mystic-shot": {
   "nome": "Mystic Shot",
   "tipo": "weapon",
   "tier": 2,
   "ativo": false
  },
  "mystic-slow": {
   "nome": "Mystic Slow",
   "tipo": "spirit",
   "tier": 2,
   "ativo": false
  },
  "mystic-vulnerability": {
   "nome": "Mystic Vulnerability",
   "tipo": "spirit",
   "tier": 2,
   "ativo": false
  },
  "opening-rounds": {
   "nome": "Opening Rounds",
   "tipo": "weapon",
   "tier": 2,
   "ativo": false
  },
  "plated-armor": {
   "nome": "Plated Armor",
   "tipo": "vitality",
   "tier": 4,
   "ativo": false
  },
  "point-blank": {
   "nome": "Point Blank",
   "tipo": "weapon",
   "tier": 3,
   "ativo": false
  },
  "quicksilver-reload": {
   "nome": "Quicksilver Reload",
   "tipo": "spirit",
   "tier": 2,
   "ativo": false
  },
  "radiant-regeneration": {
   "nome": "Radiant Regeneration",
   "tipo": "spirit",
   "tier": 3,
   "ativo": false
  },
  "rapid-recharge": {
   "nome": "Rapid Recharge",
   "tipo": "spirit",
   "tier": 3,
   "ativo": false
  },
  "rebuttal": {
   "nome": "Rebuttal",
   "tipo": "vitality",
   "tier": 1,
   "ativo": false
  },
  "refresher": {
   "nome": "Refresher",
   "tipo": "spirit",
   "tier": 4,
   "ativo": true
  },
  "rescue-beam": {
   "nome": "Rescue Beam",
   "tipo": "vitality",
   "tier": 3,
   "ativo": true
  },
  "restorative-locket": {
   "nome": "Restorative Locket",
   "tipo": "vitality",
   "tier": 2,
   "ativo": true
  },
  "restorative-shot": {
   "nome": "Restorative Shot",
   "tipo": "weapon",
   "tier": 1,
   "ativo": false
  },
  "return-fire": {
   "nome": "Return Fire",
   "tipo": "vitality",
   "tier": 2,
   "ativo": true
  },
  "ricochet": {
   "nome": "Ricochet",
   "tipo": "weapon",
   "tier": 4,
   "ativo": false
  },
  "silence-wave": {
   "nome": "Silence Wave",
   "tipo": "spirit",
   "tier": 3,
   "ativo": true
  },
  "silencer": {
   "nome": "Silencer",
   "tipo": "weapon",
   "tier": 4,
   "ativo": false
  },
  "siphon-bullets": {
   "nome": "Siphon Bullets",
   "tipo": "vitality",
   "tier": 4,
   "ativo": false
  },
  "spellslinger": {
   "nome": "Spellslinger",
   "tipo": "weapon",
   "tier": 4,
   "ativo": false
  },
  "spirit-burn": {
   "nome": "Spirit Burn",
   "tipo": "spirit",
   "tier": 4,
   "ativo": false
  },
  "spirit-lifesteal": {
   "nome": "Spirit Lifesteal",
   "tipo": "vitality",
   "tier": 2,
   "ativo": false
  },
  "spirit-rend": {
   "nome": "Spirit Rend",
   "tipo": "weapon",
   "tier": 3,
   "ativo": false
  },
  "spirit-resilience": {
   "nome": "Spirit Resilience",
   "tipo": "vitality",
   "tier": 3,
   "ativo": false
  },
  "spirit-shielding": {
   "nome": "Spirit Shielding",
   "tipo": "vitality",
   "tier": 2,
   "ativo": false
  },
  "spirit-snatch": {
   "nome": "Spirit Snatch",
   "tipo": "spirit",
   "tier": 3,
   "ativo": false
  },
  "spiritual-overflow": {
   "nome": "Spiritual Overflow",
   "tipo": "weapon",
   "tier": 4,
   "ativo": false
  },
  "stalker": {
   "nome": "Stalker",
   "tipo": "weapon",
   "tier": 2,
   "ativo": false
  },
  "stamina-mastery": {
   "nome": "Stamina Mastery",
   "tipo": "vitality",
   "tier": 3,
   "ativo": false
  },
  "superior-cooldown": {
   "nome": "Superior Cooldown",
   "tipo": "spirit",
   "tier": 3,
   "ativo": false
  },
  "superior-duration": {
   "nome": "Superior Duration",
   "tipo": "spirit",
   "tier": 3,
   "ativo": false
  },
  "surge-of-power": {
   "nome": "Surge of Power",
   "tipo": "spirit",
   "tier": 3,
   "ativo": false
  },
  "swift-striker": {
   "nome": "Swift Striker",
   "tipo": "weapon",
   "tier": 2,
   "ativo": false
  },
  "tankbuster": {
   "nome": "Tankbuster",
   "tipo": "spirit",
   "tier": 3,
   "ativo": false
  },
  "tesla-bullets": {
   "nome": "Tesla Bullets",
   "tipo": "weapon",
   "tier": 3,
   "ativo": false
  },
  "titanic-magazine": {
   "nome": "Titanic Magazine",
   "tipo": "weapon",
   "tier": 2,
   "ativo": false
  },
  "torment-pulse": {
   "nome": "Torment Pulse",
   "tipo": "spirit",
   "tier": 3,
   "ativo": false
  },
  "toxic-bullets": {
   "nome": "Toxic Bullets",
   "tipo": "weapon",
   "tier": 3,
   "ativo": false
  },
  "transcendent-cooldown": {
   "nome": "Transcendent Cooldown",
   "tipo": "spirit",
   "tier": 4,
   "ativo": false
  },
  "trophy-collector": {
   "nome": "Trophy Collector",
   "tipo": "vitality",
   "tier": 2,
   "ativo": false
  },
  "unstoppable": {
   "nome": "Unstoppable",
   "tipo": "vitality",
   "tier": 4,
   "ativo": true
  },
  "vampiric-burst": {
   "nome": "Vampiric Burst",
   "tipo": "vitality",
   "tier": 4,
   "ativo": true
  },
  "vortex-web": {
   "nome": "Vortex Web",
   "tipo": "spirit",
   "tier": 4,
   "ativo": true
  },
  "warp-stone": {
   "nome": "Warp Stone",
   "tipo": "vitality",
   "tier": 3,
   "ativo": true
  },
  "weighted-shots": {
   "nome": "Weighted Shots",
   "tipo": "weapon",
   "tier": 3,
   "ativo": false
  }
 }
};
