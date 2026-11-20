//! Milestone 3: the world (SPEC sections 7 and 8): the seeded generator,
//! level text, the tile map, where things spawn, and the camera.

mod common;

use common::{LEVEL_1, RIGHT, SUB, TILE, game, run};
use game::{EnemyKind, EnemyView, Game, LevelError, Rng, Tile};

#[test]
fn rng_matches_the_reference_values() {
    let _watch = common::watch();
    // SPEC 7.1, worked example: seed 0.
    let mut rng = Rng::new(0);
    assert_eq!(rng.state(), 0, "the seed is the state, unchanged");
    assert_eq!(rng.next_u64(), 0xE220_A839_7B1D_CDAF);
    assert_eq!(
        rng.state(),
        0x9E37_79B9_7F4A_7C15,
        "one draw adds the increment once"
    );
    assert_eq!(rng.next_u64(), 0x6E78_9E6A_A1B9_65F4);
    assert_eq!(rng.next_u64(), 0x06C4_5D18_8009_454F);

    let mut rng = Rng::new(u64::MAX);
    assert_eq!(
        rng.next_u64(),
        RNG_MAX_SEED_FIRST,
        "the state wraps around; it must not panic"
    );
}
const RNG_MAX_SEED_FIRST: u64 = 0xE4D9_7177_1B65_2C20;

#[test]
fn rng_next_u32_is_the_high_half() {
    let _watch = common::watch();
    let mut a = Rng::new(42);
    let mut b = Rng::new(42);
    for _ in 0..100 {
        assert_eq!(u64::from(a.next_u32()), b.next_u64() >> 32);
    }
    assert_eq!(a.state(), b.state(), "next_u32 consumes exactly one draw");
}

#[test]
fn rng_below_is_multiply_and_shift() {
    let _watch = common::watch();
    let mut a = Rng::new(2026);
    let mut b = Rng::new(2026);
    for n in [1u32, 2, 3, 8, 10, 100, 255, 256, 1_000_000, u32::MAX] {
        let expected = ((u64::from(b.next_u32()) * u64::from(n)) >> 32) as u32;
        let got = a.below(n);
        assert_eq!(got, expected, "below({n}) is (next_u32() * n) >> 32");
        assert!(got < n);
    }
    assert_eq!(a.below(0), 0);
    b.next_u32();
    assert_eq!(a.state(), b.state(), "below(0) still consumes one draw");

    // SPEC 7.2, worked example.
    let mut rng = Rng::new(0);
    assert_eq!(
        [rng.below(8), rng.below(8), rng.below(8), rng.below(8)],
        RNG_BELOW_8,
    );
}
const RNG_BELOW_8: [u32; 4] = [7, 3, 0, 7];

const SMALL: &str = "\
#.........
#..f....o.
#.P..w..E.
######~~##
";

#[test]
fn a_level_becomes_a_tile_map() {
    let _watch = common::watch();
    let g = game(SMALL, 1);
    assert_eq!(g.map_size(), (10, 4), "map_size is (columns, rows)");
    assert_eq!(g.tile(0, 0), Tile::Solid);
    assert_eq!(g.tile(1, 0), Tile::Empty);
    assert_eq!(g.tile(6, 3), Tile::Water);
    assert_eq!(g.tile(7, 3), Tile::Water);
    assert_eq!(g.tile(8, 3), Tile::Solid);
    for (col, row, what) in [
        (2, 2, "P"),
        (3, 1, "f"),
        (5, 2, "w"),
        (8, 1, "o"),
        (8, 2, "E"),
    ] {
        assert_eq!(
            g.tile(col, row),
            Tile::Empty,
            "the tile under `{what}` at ({col}, {row}) is empty"
        );
    }
}

#[test]
fn outside_the_map_is_rock_except_below() {
    let _watch = common::watch();
    let g = game(SMALL, 1);
    assert_eq!(g.tile(-1, 1), Tile::Solid, "left of the map is solid");
    assert_eq!(g.tile(10, 1), Tile::Solid, "right of the map is solid");
    assert_eq!(g.tile(4, -1), Tile::Solid, "above the map is solid");
    assert_eq!(g.tile(4, 4), Tile::Empty, "below the map is empty");
    assert_eq!(g.tile(-1, 4), Tile::Solid, "left wins over below");
    assert_eq!(g.tile(i32::MIN, i32::MIN), Tile::Solid);
    assert_eq!(g.tile(3, i32::MAX), Tile::Empty);
}

#[test]
fn everything_starts_where_the_spec_puts_it() {
    let _watch = common::watch();
    let g = game(SMALL, 1);
    assert_eq!(g.tick(), 0);
    let p = g.player();
    // Player box 10x14 px, centred in column 2, feet on the bottom of row 2.
    assert_eq!(
        p.x,
        2 * TILE + 3 * SUB,
        "(16 - 10) / 2 == 3 pixels from the tile's left edge"
    );
    assert_eq!(
        p.y,
        3 * TILE - 14 * SUB,
        "feet on the bottom edge of the tile"
    );
    assert_eq!((p.vx, p.vy), (0, 0));

    assert_eq!(g.enemy_count(), 2);
    assert_eq!(
        g.enemy(0),
        Some(EnemyView {
            kind: EnemyKind::Flyer,
            alive: true,
            x: 3 * TILE + 2 * SUB,
            y: 2 * TILE - 8 * SUB,
        }),
        "enemies are numbered in reading order: the flyer in row 2 comes first; its 12x8 box \
         stands on the bottom edge of its tile"
    );
    assert_eq!(
        g.enemy(1),
        Some(EnemyView {
            kind: EnemyKind::Walker,
            alive: true,
            x: 5 * TILE + 2 * SUB,
            y: 3 * TILE - 10 * SUB,
        })
    );
    assert_eq!(g.enemy(2), None);
    assert_eq!(g.sparks_left(), 1);
}

#[test]
fn level_text_may_use_windows_line_endings_and_trailing_blank_lines() {
    let _watch = common::watch();
    let unix = game(SMALL, 5);
    let windows = game(&SMALL.replace('\n', "\r\n"), 5);
    let padded = game(&format!("{SMALL}\n\n"), 5);
    for other in [&windows, &padded] {
        assert_eq!(other.map_size(), unix.map_size());
        assert_eq!(other.state_hash(), unix.state_hash());
    }
}

#[test]
fn bad_levels_are_rejected_with_the_reason() {
    let _watch = common::watch();
    #[track_caller]
    fn check(level: &str, expected: LevelError) {
        match Game::new(level, 1) {
            Ok(_) => panic!("this level must be rejected with {expected:?}:\n{level}"),
            Err(e) => assert_eq!(e, expected, "wrong error for:\n{level}"),
        }
    }
    check("", LevelError::Empty);
    check("\n\n", LevelError::Empty);
    check("P.E\n##\n", LevelError::Ragged { line: 2 });
    check("P.E\n###\n####\n", LevelError::Ragged { line: 3 });
    check(
        "P.E\n#?#\n",
        LevelError::UnknownChar {
            line: 2,
            column: 2,
            ch: '?',
        },
    );
    check(
        "P.Eé\n####\n",
        LevelError::UnknownChar {
            line: 1,
            column: 4,
            ch: 'é',
        },
    );
    check("..E\n###\n", LevelError::NoPlayer);
    check("P.E\n#P#\n", LevelError::ManyPlayers);
    check("P..\n###\n", LevelError::NoExit);
    check("PEE\n###\n", LevelError::ManyExits);
    let wide = format!("PE{}\n", ".".repeat(255));
    check(&wide, LevelError::TooLarge);
    let tall = format!("PE\n{}", "..\n".repeat(256));
    check(&tall, LevelError::TooLarge);
    let largest = format!(
        "PE{}\n{}",
        ".".repeat(254),
        format!("{}\n", ".".repeat(256)).repeat(255)
    );
    assert!(
        Game::new(&largest, 1).is_ok(),
        "256 columns by 256 rows is the largest level and must load"
    );
}

#[test]
fn the_shipped_level_loads() {
    let _watch = common::watch();
    let g = game(LEVEL_1, 1);
    assert_eq!(g.map_size(), (80, 18));
    assert_eq!(g.enemy_count(), 5);
    assert_eq!(g.sparks_left(), 14);
    let kinds: Vec<EnemyKind> = (0..5)
        .map(|i| g.enemy(i).expect("five enemies").kind)
        .collect();
    assert_eq!(
        kinds,
        [
            EnemyKind::Flyer,
            EnemyKind::Flyer,
            EnemyKind::Walker,
            EnemyKind::Walker,
            EnemyKind::Walker
        ],
        "reading order: top row first, left to right within a row"
    );
}

#[test]
fn a_map_smaller_than_the_screen_is_centred() {
    let _watch = common::watch();
    // 10x4 tiles is 160x64 pixels on a 320x180 screen.
    let mut g = game(SMALL, 1);
    assert_eq!(g.camera(), (-80, -58), "((160 - 320) / 2, (64 - 180) / 2)");
    run(&mut g, RIGHT, 20);
    assert_eq!(
        g.camera(),
        (-80, -58),
        "the camera of a small map never moves"
    );
}

#[test]
fn the_camera_follows_the_player_and_stops_at_the_map_edges() {
    let _watch = common::watch();
    // 40 columns (640 px) by 14 rows (224 px); the player starts at the left.
    let mut level = String::new();
    for row in 0..14 {
        let line = match row {
            12 => format!("#P{}E#", ".".repeat(36)),
            13 => "#".repeat(40),
            _ => format!("#{}#", ".".repeat(38)),
        };
        level.push_str(&line);
        level.push('\n');
    }
    let mut g = game(&level, 1);
    assert_eq!(
        g.camera(),
        (0, 44),
        "at the left edge x is clamped to 0; at the bottom y is clamped to 224 - 180"
    );
    let mut moved = false;
    for _ in 0..400 {
        g.step(RIGHT);
        let p = g.player();
        let center_x = (p.x + 5 * SUB) >> 8;
        let center_y = (p.y + 7 * SUB) >> 8;
        let expected = (
            (center_x - 160).clamp(0, 640 - 320),
            (center_y - 90).clamp(0, 224 - 180),
        );
        assert_eq!(
            g.camera(),
            expected,
            "tick {}: the camera centres on the middle of the player's box, then clamps",
            g.tick()
        );
        moved |= expected.0 > 0 && expected.0 < 320;
    }
    assert!(
        moved,
        "the player ran across the map, so the camera must have scrolled"
    );
    assert_eq!(
        g.camera().0,
        320,
        "at the right edge x is clamped to 640 - 320"
    );
}
