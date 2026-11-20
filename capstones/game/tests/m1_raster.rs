//! Milestone 1: the framebuffer and the rasteriser (SPEC sections 3 to 5).
//!
//! Every expected value in this file can be worked out from the SPEC with
//! pencil and paper; the checksums are FNV-1a (SPEC 3.3) over pictures that
//! the SPEC rules define completely.

mod common;

use common::pixel_checksum;
use game::{ArtError, Flip, Framebuffer, Sprite};

const BLACK: u32 = 0xFF00_0000;
const WHITE: u32 = 0xFFFF_FFFF;
const RED: u32 = 0xFFFF_0000;
const GREEN: u32 = 0xFF00_FF00;
const BLUE: u32 = 0xFF00_00FF;

/// The buffer as rows of characters: `#` where the pixel equals `ink`,
/// `.` elsewhere. Makes failures readable.
fn picture(fb: &Framebuffer, ink: u32) -> String {
    let mut out = String::new();
    for row in fb.pixels().chunks(fb.width()) {
        out.extend(row.iter().map(|&p| if p == ink { '#' } else { '.' }));
        out.push('\n');
    }
    out
}

#[track_caller]
fn assert_picture(fb: &Framebuffer, ink: u32, expected: &[&str]) {
    let expected: String = expected.iter().flat_map(|row| [row, "\n"]).collect();
    let actual = picture(fb, ink);
    assert!(
        actual == expected,
        "the picture is wrong.\nexpected:\n{expected}\nactual:\n{actual}"
    );
}

#[test]
fn a_new_buffer_is_opaque_black() {
    let _watch = common::watch();
    let fb = Framebuffer::new(7, 3);
    assert_eq!((fb.width(), fb.height()), (7, 3));
    assert_eq!(
        fb.pixels().len(),
        21,
        "pixels() holds width * height values"
    );
    assert!(
        fb.pixels().iter().all(|&p| p == BLACK),
        "a new buffer is 0xFF000000 everywhere"
    );
}

#[test]
fn pixels_are_stored_row_by_row_from_the_top() {
    let _watch = common::watch();
    let mut fb = Framebuffer::new(4, 3);
    fb.set_pixel(1, 2, RED);
    assert_eq!(
        fb.pixels()[2 * 4 + 1],
        RED,
        "pixel (x, y) lives at index y * width + x"
    );
    assert_eq!(fb.get_pixel(1, 2), Some(RED));
    assert_eq!(fb.get_pixel(2, 1), Some(BLACK));
}

#[test]
fn set_and_get_ignore_points_outside_the_buffer() {
    let _watch = common::watch();
    let mut fb = Framebuffer::new(4, 3);
    for (x, y) in [
        (-1, 0),
        (0, -1),
        (4, 0),
        (0, 3),
        (i32::MIN, i32::MIN),
        (i32::MAX, i32::MAX),
    ] {
        fb.set_pixel(x, y, RED);
        fb.blend_pixel(x, y, RED, 255);
        assert_eq!(
            fb.get_pixel(x, y),
            None,
            "({x}, {y}) is outside a 4x3 buffer"
        );
    }
    assert!(
        fb.pixels().iter().all(|&p| p == BLACK),
        "drawing outside the buffer must not change any pixel (check for wrap-around)"
    );
}

#[test]
fn every_drawing_call_stores_opaque_pixels() {
    let _watch = common::watch();
    let mut fb = Framebuffer::new(8, 8);
    fb.clear(0x0012_3456);
    assert_eq!(
        fb.get_pixel(0, 0),
        Some(0xFF12_3456),
        "clear forces alpha to 0xFF"
    );
    fb.set_pixel(1, 1, 0x0000_00FF);
    assert_eq!(
        fb.get_pixel(1, 1),
        Some(BLUE),
        "set_pixel forces alpha to 0xFF"
    );
    fb.fill_rect(2, 2, 2, 2, 0x7F00_FF00);
    assert_eq!(
        fb.get_pixel(3, 3),
        Some(GREEN),
        "fill_rect forces alpha to 0xFF"
    );
    fb.line(0, 7, 7, 7, 0x00FF_0000);
    assert_eq!(fb.get_pixel(4, 7), Some(RED), "line forces alpha to 0xFF");
    fb.text("I", 0, 0, 0x00FF_FFFF);
    assert_eq!(fb.get_pixel(2, 3), Some(WHITE), "text forces alpha to 0xFF");
}

#[test]
fn fill_rect_covers_a_half_open_rectangle() {
    let _watch = common::watch();
    let mut fb = Framebuffer::new(8, 5);
    fb.fill_rect(2, 1, 3, 2, WHITE);
    assert_picture(
        &fb,
        WHITE,
        &["........", "..###...", "..###...", "........", "........"],
    );
}

#[test]
fn fill_rect_is_clipped_on_every_side() {
    let _watch = common::watch();
    let mut fb = Framebuffer::new(6, 4);
    fb.fill_rect(-2, -1, 4, 3, WHITE);
    fb.fill_rect(4, 2, 10, 10, WHITE);
    assert_picture(&fb, WHITE, &["##....", "##....", "....##", "....##"]);
}

#[test]
fn fill_rect_with_no_area_draws_nothing() {
    let _watch = common::watch();
    let mut fb = Framebuffer::new(6, 4);
    fb.fill_rect(1, 1, 0, 3, WHITE);
    fb.fill_rect(1, 1, 3, 0, WHITE);
    fb.fill_rect(3, 3, -2, -2, WHITE);
    fb.fill_rect(6, 0, 5, 5, WHITE);
    fb.fill_rect(0, 4, 5, 5, WHITE);
    fb.fill_rect(-5, -5, 5, 5, WHITE);
    assert!(
        fb.pixels().iter().all(|&p| p == BLACK),
        "empty, negative and fully off-screen rectangles change nothing:\n{}",
        picture(&fb, WHITE)
    );
}

#[test]
fn fill_rect_survives_extreme_arguments() {
    let _watch = common::watch();
    let mut fb = Framebuffer::new(6, 4);
    // x + w does not fit in an i32 here. The rectangle still reaches the
    // right and bottom edges; a wrapped sum would draw nothing or panic.
    fb.fill_rect(2, 1, i32::MAX, i32::MAX, RED);
    assert_picture(&fb, RED, &["......", "..####", "..####", "..####"]);

    fb.fill_rect(-10, -10, i32::MAX, i32::MAX, RED);
    assert!(
        fb.pixels().iter().all(|&p| p == RED),
        "a rectangle from (-10, -10) with huge sides covers every pixel"
    );
    fb.fill_rect(i32::MAX, i32::MAX, i32::MAX, i32::MAX, GREEN);
    fb.fill_rect(i32::MIN, i32::MIN, 10, 10, GREEN);
    fb.fill_rect(0, 0, i32::MIN, i32::MIN, GREEN);
    assert!(
        fb.pixels().iter().all(|&p| p == RED),
        "rectangles that lie outside the buffer change nothing"
    );
    fb.fill_rect(i32::MIN, 1, i32::MAX, 1, GREEN);
    assert_eq!(
        fb.get_pixel(0, 1),
        Some(RED),
        "x = i32::MIN with w = i32::MAX ends at x = -1, left of the buffer"
    );
}

#[test]
fn line_follows_the_worked_example() {
    let _watch = common::watch();
    // SPEC 4.3, worked example: (0, 0) to (5, 2).
    let mut fb = Framebuffer::new(7, 4);
    fb.line(0, 0, 5, 2, WHITE);
    assert_picture(&fb, WHITE, &["##.....", "..##...", "....##.", "......."]);
}

#[test]
fn line_draws_both_end_points_in_every_direction() {
    let _watch = common::watch();
    type Case = ((i32, i32), (i32, i32), &'static [&'static str]);
    let cases: [Case; 6] = [
        (
            (1, 1),
            (1, 1),
            &[".....", ".#...", ".....", ".....", "....."],
        ),
        (
            (0, 2),
            (4, 2),
            &[".....", ".....", "#####", ".....", "....."],
        ),
        (
            (3, 4),
            (3, 0),
            &["...#.", "...#.", "...#.", "...#.", "...#."],
        ),
        (
            (4, 4),
            (0, 0),
            &["#....", ".#...", "..#..", "...#.", "....#"],
        ),
        (
            (0, 4),
            (2, 0),
            &["..#..", "..#..", ".#...", ".#...", "#...."],
        ),
        (
            (4, 1),
            (0, 3),
            &[".....", "....#", "..##.", "##...", "....."],
        ),
    ];
    for (from, to, expected) in cases {
        let mut fb = Framebuffer::new(5, 5);
        fb.line(from.0, from.1, to.0, to.1, WHITE);
        let expected_text: String = expected.iter().flat_map(|row| [row, "\n"]).collect();
        assert!(
            picture(&fb, WHITE) == expected_text,
            "line from {from:?} to {to:?}.\nexpected:\n{expected_text}\nactual:\n{}",
            picture(&fb, WHITE)
        );
    }
}

#[test]
fn line_is_clipped_pixel_by_pixel() {
    let _watch = common::watch();
    // The visible part of a clipped line is exactly the part of the full
    // line that falls inside the buffer: clipping must not move the pixels.
    let mut big = Framebuffer::new(40, 40);
    big.line(2, 5, 37, 31, WHITE);
    let mut small = Framebuffer::new(12, 9);
    // The same line, seen through a 12x9 window whose corner is at (14, 12).
    small.line(2 - 14, 5 - 12, 37 - 14, 31 - 12, WHITE);
    for y in 0..9 {
        for x in 0..12 {
            assert_eq!(
                small.get_pixel(x, y),
                big.get_pixel(x + 14, y + 12),
                "pixel ({x}, {y}) of the clipped line differs from the unclipped line"
            );
        }
    }
    assert!(
        small.pixels().contains(&WHITE),
        "the line crosses the window, so some pixels must be set"
    );
}

#[test]
fn a_fan_of_lines_has_the_specified_checksum() {
    let _watch = common::watch();
    // Sixteen lines from the centre of a 33x33 buffer to points on its
    // border, each in its own colour.
    let mut fb = Framebuffer::new(33, 33);
    let ends = [
        (32, 16),
        (32, 24),
        (32, 32),
        (24, 32),
        (16, 32),
        (8, 32),
        (0, 32),
        (0, 24),
        (0, 16),
        (0, 8),
        (0, 0),
        (8, 0),
        (16, 0),
        (24, 0),
        (32, 0),
        (32, 8),
    ];
    for (i, (x, y)) in ends.into_iter().enumerate() {
        fb.line(16, 16, x, y, 0xFF00_0000 | (0x0010_2030 * (i as u32 + 1)));
    }
    assert_eq!(
        fb.checksum(),
        FAN_CHECKSUM,
        "the fan of lines differs from SPEC 4.3; check the two `if` tests and their order"
    );
}
const FAN_CHECKSUM: u64 = 16975490615298824094;

fn arrow() -> Sprite {
    // 3x2, every pixel different, one transparent.
    Sprite::new(3, 2, vec![RED, GREEN, BLUE, 0, WHITE, 0xFF11_2233]).expect("3 * 2 == 6 pixels")
}

#[test]
fn sprite_new_checks_its_arguments() {
    let _watch = common::watch();
    assert!(Sprite::new(2, 2, vec![0; 4]).is_some());
    assert!(
        Sprite::new(2, 2, vec![0; 3]).is_none(),
        "3 pixels cannot fill 2x2"
    );
    assert!(
        Sprite::new(0, 2, vec![]).is_none(),
        "a sprite has at least one column"
    );
    assert!(
        Sprite::new(2, 0, vec![]).is_none(),
        "a sprite has at least one row"
    );
    let s = arrow();
    assert_eq!((s.width(), s.height()), (3, 2));
    assert_eq!(s.pixel(2, 0), Some(BLUE));
    assert_eq!(
        s.pixel(0, 1),
        Some(0),
        "sprites keep transparent pixels as they are"
    );
    assert_eq!(s.pixel(3, 0), None);
    assert_eq!(s.pixel(0, 2), None);
}

#[test]
fn from_art_reads_rows_of_palette_characters() {
    let _watch = common::watch();
    let palette = [('.', 0), ('r', RED), ('w', WHITE)];
    let s = Sprite::from_art("\nr.w\n.w.\n", &palette).expect("valid art");
    assert_eq!((s.width(), s.height()), (3, 2), "blank lines are not rows");
    assert_eq!(s.pixel(0, 0), Some(RED));
    assert_eq!(s.pixel(1, 0), Some(0));
    assert_eq!(s.pixel(2, 0), Some(WHITE));
    assert_eq!(s.pixel(1, 1), Some(WHITE));

    let crlf = Sprite::from_art("r.w\r\n.w.\r\n", &palette).expect("Windows line endings");
    assert_eq!(
        (crlf.width(), crlf.height()),
        (3, 2),
        "a trailing \\r is not a pixel"
    );
}

#[test]
fn from_art_reports_what_is_wrong() {
    let _watch = common::watch();
    let palette = [('.', 0), ('r', RED)];
    assert_eq!(Sprite::from_art("", &palette).err(), Some(ArtError::Empty));
    assert_eq!(
        Sprite::from_art("\n\n", &palette).err(),
        Some(ArtError::Empty)
    );
    assert_eq!(
        Sprite::from_art("r.\nr\n", &palette).err(),
        Some(ArtError::Ragged { line: 2 }),
        "line 2 is shorter than line 1"
    );
    assert_eq!(
        Sprite::from_art("\nr.\nrx\n", &palette).err(),
        Some(ArtError::UnknownChar { line: 3, ch: 'x' }),
        "lines are numbered from 1 and blank lines count"
    );
}

#[test]
fn blit_copies_opaque_pixels_and_skips_transparent_ones() {
    let _watch = common::watch();
    let mut fb = Framebuffer::new(5, 4);
    fb.clear(0xFF55_5555);
    fb.blit(&arrow(), 1, 1, Flip::NONE);
    let bg = 0xFF55_5555;
    #[rustfmt::skip]
    let expected = [
        bg, bg,  bg,    bg,          bg,
        bg, RED, GREEN, BLUE,        bg,
        bg, bg,  WHITE, 0xFF11_2233, bg,
        bg, bg,  bg,    bg,          bg,
    ];
    assert_eq!(
        fb.pixels(),
        expected,
        "the transparent pixel at sprite (0, 1) must leave the background alone"
    );
}

#[test]
fn blit_forces_alpha_and_treats_only_zero_alpha_as_transparent() {
    let _watch = common::watch();
    let mut fb = Framebuffer::new(3, 1);
    let s = Sprite::new(3, 1, vec![0x0100_00FF, 0x00FF_FFFF, 0x8000_FF00]).expect("3 pixels");
    fb.blit(&s, 0, 0, Flip::NONE);
    assert_eq!(
        fb.pixels(),
        [BLUE, BLACK, GREEN],
        "alpha 0x01 and 0x80 are opaque (stored with alpha 0xFF); alpha 0x00 is skipped even \
         when the colour bits are set"
    );
}

#[test]
fn blit_mirrors_as_flip_says() {
    let _watch = common::watch();
    let t = 0xFF55_5555; // background showing through
    let cases = [
        (Flip::NONE, [RED, GREEN, BLUE, t, WHITE, 0xFF11_2233]),
        (Flip::X, [BLUE, GREEN, RED, 0xFF11_2233, WHITE, t]),
        (Flip::Y, [t, WHITE, 0xFF11_2233, RED, GREEN, BLUE]),
        (Flip::BOTH, [0xFF11_2233, WHITE, t, BLUE, GREEN, RED]),
    ];
    for (flip, expected) in cases {
        let mut fb = Framebuffer::new(3, 2);
        fb.clear(t);
        fb.blit(&arrow(), 0, 0, flip);
        assert_eq!(fb.pixels(), expected, "blit with {flip:?}");
    }
}

#[test]
fn blit_is_clipped_on_every_side_also_when_flipped() {
    let _watch = common::watch();
    // A 4x4 sprite whose pixel (x, y) has the colour 0xFF0000xy.
    let s = Sprite::new(
        4,
        4,
        (0..16)
            .map(|i| 0xFF00_0000 | (i % 4) << 4 | (i / 4))
            .collect(),
    )
    .expect("16 pixels");
    for flip in [Flip::NONE, Flip::X, Flip::Y, Flip::BOTH] {
        for (ox, oy) in [(-2, -1), (3, 3), (-3, 4), (2, -3), (-4, 0), (0, 6), (1, 1)] {
            let mut fb = Framebuffer::new(5, 6);
            fb.blit(&s, ox, oy, flip);
            for y in 0..6 {
                for x in 0..5 {
                    let (sx, sy) = (x - ox, y - oy);
                    let expected = if (0..4).contains(&sx) && (0..4).contains(&sy) {
                        let sx = if flip.x { 3 - sx } else { sx };
                        let sy = if flip.y { 3 - sy } else { sy };
                        0xFF00_0000 | (sx as u32) << 4 | sy as u32
                    } else {
                        BLACK
                    };
                    assert_eq!(
                        fb.get_pixel(x, y),
                        Some(expected),
                        "sprite at ({ox}, {oy}) with {flip:?}: wrong pixel at ({x}, {y})"
                    );
                }
            }
        }
    }
    let mut fb = Framebuffer::new(5, 6);
    fb.blit(&s, i32::MAX, i32::MAX, Flip::NONE);
    fb.blit(&s, i32::MIN, i32::MIN, Flip::BOTH);
    assert!(
        fb.pixels().iter().all(|&p| p == BLACK),
        "a sprite far outside the buffer draws nothing and must not overflow"
    );
}

#[test]
fn blend_pixel_mixes_each_channel_with_rounding() {
    let _watch = common::watch();
    let mut fb = Framebuffer::new(4, 1);
    fb.blend_pixel(0, 0, WHITE, 0);
    fb.blend_pixel(1, 0, WHITE, 255);
    fb.blend_pixel(2, 0, WHITE, 128);
    fb.set_pixel(3, 0, 0xFF64_C800);
    fb.blend_pixel(3, 0, 0x00FF_0A32, 77);
    assert_eq!(
        fb.get_pixel(0, 0),
        Some(BLACK),
        "alpha 0 keeps the old pixel"
    );
    assert_eq!(
        fb.get_pixel(1, 0),
        Some(WHITE),
        "alpha 255 takes the new colour"
    );
    assert_eq!(
        fb.get_pixel(2, 0),
        Some(0xFF80_8080),
        "(255 * 128 + 0 * 127 + 127) / 255 == 128"
    );
    // SPEC 4.5, worked example:
    //   red   (255 * 77 + 100 * 178 + 127) / 255 == 147
    //   green ( 10 * 77 + 200 * 178 + 127) / 255 == 143
    //   blue  ( 50 * 77 +   0 * 178 + 127) / 255 ==  15
    assert_eq!(fb.get_pixel(3, 0), Some(0xFF93_8F0F));
}

#[test]
fn text_draws_the_glyphs_of_the_font_file() {
    let _watch = common::watch();
    // SPEC section 5, worked example.
    let mut fb = Framebuffer::new(13, 9);
    let end = fb.text("A1", 1, 1, WHITE);
    assert_eq!(end, 13, "two characters advance the pen by 2 * 6 pixels");
    assert_picture(
        &fb,
        WHITE,
        &[
            ".............",
            "..###....#...",
            ".#...#..##...",
            ".#...#...#...",
            ".#####...#...",
            ".#...#...#...",
            ".#...#...#...",
            ".#...#..###..",
            ".............",
        ],
    );
}

#[test]
fn text_handles_space_lower_case_and_unknown_characters() {
    let _watch = common::watch();
    let mut upper = Framebuffer::new(40, 7);
    let mut lower = Framebuffer::new(40, 7);
    assert_eq!(upper.text("AB Z", 0, 0, WHITE), 24);
    assert_eq!(lower.text("ab z", 0, 0, WHITE), 24);
    assert_eq!(
        picture(&upper, WHITE),
        picture(&lower, WHITE),
        "lower-case ASCII letters use the upper-case glyphs"
    );
    assert!(
        (0..7).all(|y| (12..18).all(|x| upper.get_pixel(x, y) == Some(BLACK))),
        "a space draws nothing but still advances the pen"
    );

    let mut fb = Framebuffer::new(12, 7);
    assert_eq!(
        fb.text("\u{e9}~", 0, 0, WHITE),
        12,
        "one advance per `char`, not per byte"
    );
    let boxed = [
        "#####.", "#...#.", "#...#.", "#...#.", "#...#.", "#...#.", "#####.",
    ];
    let expected: Vec<String> = boxed.iter().map(|row| row.repeat(2)).collect();
    let expected: Vec<&str> = expected.iter().map(String::as_str).collect();
    assert_picture(&fb, WHITE, &expected);

    assert_eq!(
        fb.text("", 7, 0, WHITE),
        7,
        "an empty string leaves the pen where it was"
    );
}

#[test]
fn text_is_clipped() {
    let _watch = common::watch();
    let mut big = Framebuffer::new(60, 30);
    big.text("NIB 42!", 9, 11, WHITE);
    let mut small = Framebuffer::new(20, 5);
    // The same text seen through a 20x5 window whose corner is at (14, 12).
    let end = small.text("NIB 42!", 9 - 14, 11 - 12, WHITE);
    assert_eq!(
        end,
        9 - 14 + 7 * 6,
        "the return value does not depend on clipping"
    );
    for y in 0..5 {
        for x in 0..20 {
            assert_eq!(
                small.get_pixel(x, y),
                big.get_pixel(x + 14, y + 12),
                "pixel ({x}, {y}) of the clipped text differs from the unclipped text"
            );
        }
    }
}

#[test]
fn the_whole_font_has_the_specified_checksum() {
    let _watch = common::watch();
    let mut fb = Framebuffer::new(160, 40);
    fb.text("0123456789 .,:!?-+/'()%=*", 2, 2, 0xFFF4_F0E0);
    fb.text("ABCDEFGHIJKLMNOPQRSTUVWXYZ", 2, 12, 0xFFFF_C94A);
    fb.text("abcdefghijklmnopqrstuvwxyz", 2, 22, 0xFF9F_DCFF);
    fb.text("#@[]{}<>\\_^&$;|~`\"", 2, 32, 0xFFEE_7B2E);
    assert_eq!(
        fb.checksum(),
        FONT_CHECKSUM,
        "some glyph differs from assets/font5x7.txt (or the pen advance is not 6)"
    );
}
const FONT_CHECKSUM: u64 = 9882842875989206531;

#[test]
fn checksum_is_fnv1a_over_little_endian_pixels() {
    let _watch = common::watch();
    let fb = Framebuffer::new(1, 1);
    // One pixel 0xFF000000 is the bytes 00 00 00 FF.
    assert_eq!(
        fb.checksum(),
        0x4d25_077f_9dcd_5758,
        "see the worked example in SPEC 3.3"
    );

    let mut fb = Framebuffer::new(9, 5);
    fb.fill_rect(1, 1, 4, 3, 0xFF12_3456);
    fb.line(0, 4, 8, 0, 0xFFAB_CDEF);
    assert_eq!(
        fb.checksum(),
        pixel_checksum(fb.pixels()),
        "checksum() must hash pixels() in order, four little-endian bytes per pixel"
    );
}

#[test]
fn write_ppm_produces_a_binary_p6_file() {
    let _watch = common::watch();
    let mut fb = Framebuffer::new(2, 2);
    fb.set_pixel(0, 0, 0xFF01_0203);
    fb.set_pixel(1, 0, 0xFFFF_FEFD);
    fb.set_pixel(1, 1, 0xFF0A_0B0C);
    let mut out = Vec::new();
    fb.write_ppm(&mut out)
        .expect("writing to a Vec cannot fail");
    let mut expected = b"P6\n2 2\n255\n".to_vec();
    expected.extend([1, 2, 3, 0xFF, 0xFE, 0xFD, 0, 0, 0, 0x0A, 0x0B, 0x0C]);
    assert_eq!(
        out, expected,
        "header `P6\\n2 2\\n255\\n`, then red, green, blue per pixel"
    );

    let big = Framebuffer::new(320, 180);
    let mut out = Vec::new();
    big.write_ppm(&mut out)
        .expect("writing to a Vec cannot fail");
    assert_eq!(out.len(), "P6\n320 180\n255\n".len() + 320 * 180 * 3);
}
