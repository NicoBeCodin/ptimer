//! Small, fixed-cell animation frames for the built-in art gallery.
//!
//! The focus spinner and sky cloud contain MIT-licensed elements credited in
//! `THIRD_PARTY_NOTICES.md`. The surrounding scenes are original to ptimer.

const BRAILLE_SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

const TOMATO: [&str; 4] = [
    r#"         __
     .-\/_ _\/-.
    /  /o\ /o\  \
   |     (_)     |
    \  `-...-'  /
     `-._____.-'"#,
    r#"      .  __
     .-\/_ _\/-.
    /  /o\ /o\  \
   |     (_)     |
    \  `-...-'  /
     `-._____.-'"#,
    r#"     *   __   *
     .-\/_ _\/-.
    /  /-\ /-\  \
   |     (_)     |
    \  `-...-'  /
     `-._____.-'"#,
    r#"         __  .
     .-\/_ _\/-.
    /  /o\ /o\  \
   |     (_)     |
    \  `-...-'  /
     `-._____.-'"#,
];

const COFFEE: [&str; 6] = [
    r#"       (  (
        )  )
      .------.
      |      |]
      \      /
       `----'"#,
    r#"        ( (
       (   )
      .------.
      |      |]
      \      /
       `----'"#,
    r#"       )  )
        (  (
      .------.
      |      |]
      \      /
       `----'"#,
    r#"      (   (
       )   )
      .------.
      |      |]
      \      /
       `----'"#,
    r#"       ( (
        ) )
      .------.
      |      |]
      \      /
       `----'"#,
    r#"        )  )
       (  (
      .------.
      |      |]
      \      /
       `----'"#,
];

const CAT: [&str; 6] = [
    r#"       /\_/\\
      ( o.o )    _
       > ^ <   _/ )
      /     \_/  /
     (  | |     /
      \_) (_/--'"#,
    r#"       /\_/\\
      ( o.o )   __
       > ^ <  _/  )
      /     \/   /
     (  | |     /
      \_) (_/--'"#,
    r#"       /\_/\\
      ( -.- )  ___
       > ^ < _/   )
      /     \/   /
     (  | |     /
      \_) (_/--'"#,
    r#"       /\_/\\
      ( o.o )  __
       > ^ < _/  \_
      /     \/     )
     (  | |       /
      \_) (_/----'"#,
    r#"       /\_/\\
      ( o.o ) _
       > ^ </  \__
      /     \     )
     (  | |      /
      \_) (_/---'"#,
    r#"       /\_/\\
      ( ^.^ )    _
       > ^ <   _/ )
      /     \_/  /
     (  | |     /
      \_) (_/--'"#,
];

const PLANT: [&str; 6] = [
    r#"



        .
      .---.
      '---'"#,
    r#"


        |
        |
      .---.
      '---'"#,
    r#"

       \|/
        |
        |
      .---.
      '---'"#,
    r#"
       _
     _/ \_
       \|/
        |
      .---.
      '---'"#,
    r#"        .
       \|/
     _/|\_
    /  |  \
       |
      .---.
      '---'"#,
    r#"      . * .
       \|/
    --< * >--
       /|\
        |
      .---.
      '---'"#,
];

const HOURGLASS: [&str; 4] = [
    r#"      _______
      \...../
       \.../
        \./
        / \
       /   \
      /_____\"#,
    r#"      _______
      \ ... /
       \ . /
        \ /
        /·\
       /   \
      /_____\"#,
    r#"      _______
      \  .  /
       \   /
        \ /
        / \
       /···\
      /_____\"#,
    r#"      _______
      \     /
       \   /
        \ /
        / \
       /.....\
      /_____\"#,
];

const FIREPLACE: [&str; 4] = [
    r#"      .========.
      |  (  )  |
      | ( /\ ) |
      |  /  \  |
      | /_/\_\ |
      |========|
       /_====_\"#,
    r#"      .========.
      |   ()   |
      |  /\(   |
      | (  )\  |
      | /\/\_\ |
      |========|
       /_====_\"#,
    r#"      .========.
      | ( )   |
      |  )(   |
      | /  \  |
      |/_/\_\ |
      |========|
       /_====_\"#,
    r#"      .========.
      |  (())  |
      |   /\   |
      |  /  \  |
      | /_/\_\ |
      |========|
       /_====_\"#,
];

pub fn tomato(frame: usize) -> String {
    normalize(TOMATO[frame % TOMATO.len()], 19, 6)
}

pub fn coffee(frame: usize) -> String {
    normalize(COFFEE[frame % COFFEE.len()], 16, 6)
}

pub fn cat(frame: usize) -> String {
    normalize(CAT[frame % CAT.len()], 22, 6)
}

pub fn plant(frame: usize) -> String {
    normalize(PLANT[frame % PLANT.len()], 18, 7)
}

pub fn hourglass(frame: usize) -> String {
    normalize(HOURGLASS[frame % HOURGLASS.len()], 18, 7)
}

pub fn fireplace(frame: usize) -> String {
    normalize(FIREPLACE[frame % FIREPLACE.len()], 20, 7)
}

pub fn focus(frame: usize) -> String {
    let spinner = BRAILLE_SPINNER[frame % BRAILLE_SPINNER.len()];
    let pulse = ["·  ·  ·", "•  ·  ·", "·  •  ·", "·  ·  •"][frame % 4];
    format!(
        "       .----------.\n     .'     {spinner}      '.\n    /    F O C U S     \\\n   |      {pulse}       |\n    \\                /\n     '._          _.'\n        '--------'"
    )
}

pub fn sky(frame: usize) -> String {
    const WIDTH: usize = 34;
    const CLOUD: [&str; 3] = [" .--.", "(    )", " `--'"];
    let mut canvas = vec![vec![' '; WIDTH]; 7];
    let cloud_x = frame % (WIDTH + 8);
    let bird_x = (frame * 2 + 18) % (WIDTH + 5);
    blit(&mut canvas, &CLOUD, cloud_x as isize - 7, 2);
    blit(&mut canvas, &["~v~"], bird_x as isize - 3, frame / 5 % 2);
    let horizon = r"__  _/\_    __/\__   _/\_  ___";
    blit(&mut canvas, &[horizon], 0, 6);
    canvas
        .into_iter()
        .map(|row| row.into_iter().collect::<String>().trim_end().to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn aquarium(frame: usize) -> String {
    const WIDTH: usize = 34;
    let mut canvas = vec![vec![' '; WIDTH]; 7];
    let fish_x = frame % (WIDTH + 7);
    let small_fish_x = (WIDTH + 4) as isize - (frame * 2 % (WIDTH + 8)) as isize;
    blit(&mut canvas, &["<`)))><"], fish_x as isize - 7, 2);
    blit(&mut canvas, &["<><"], small_fish_x, 4);
    let bubble_x = 8 + frame * 5 % 18;
    let bubble_y = 4usize.saturating_sub(frame / 2 % 5);
    blit(&mut canvas, &["o"], bubble_x as isize, bubble_y);
    blit(&mut canvas, &["~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~"], 2, 6);
    canvas_to_string(canvas)
}

pub fn rocket(frame: usize) -> String {
    const WIDTH: usize = 22;
    let mut canvas = vec![vec![' '; WIDTH]; 8];
    let y = frame / 3 % 2;
    let rocket = [
        "       /\\",
        "      /  \\",
        "     | PT |",
        "     |____|",
        "    /|    |\\",
    ];
    blit(&mut canvas, &rocket, 0, y);
    let flame = if frame.is_multiple_of(2) {
        "      /\\"
    } else {
        "      \\/"
    };
    blit(&mut canvas, &[flame], 0, y + 5);
    if frame.is_multiple_of(3) {
        blit(&mut canvas, &["  .              *"], 0, 1);
    } else {
        blit(&mut canvas, &["       .      *"], 0, 0);
    }
    canvas_to_string(canvas)
}

pub fn grand_cat(frame: usize) -> String {
    let (left_eye, right_eye) = if frame % 7 == 5 {
        ("—", "—")
    } else {
        ("●", "●")
    };
    let whisker = if frame.is_multiple_of(2) { "≋" } else { "~" };
    normalize(
        &format!(
            r#"             /\                            /\
            /  \__________________________/  \
           /                                  \
          /       .--------.    .--------.      \
         |       /    {left_eye}     \  /    {right_eye}     \      |
         |       \          /  \          /      |
         |        '--------' /\ '--------'       |
         |                  /  \                  |
    {whisker}{whisker}{whisker}  |              .-`  '-.              |  {whisker}{whisker}{whisker}
          \             `-.__.-'             /
           '._                              _.'
              '--------------------------'"#
        ),
        62,
        12,
    )
}

pub fn grand_castle(frame: usize) -> String {
    let flag = [">>>", " >>", "  >", " >>"][frame % 4];
    normalize(
        &format!(
            r#"                          |{flag}
                          |
                  _   _  _|_  _   _
                 | |_| ||   || |_| |
                 |              _  |
             _   |  []   []    | | |   _
            | |_|                |_| |_| |
            |     []    ____    []       |
            |          |    |            |
       _____|__________| __ |____________|_____
      /  /  /  /  /   |/  \|   \  \  \  \  \
     /__/__/__/__/_____/____\____\__\__\__\__\
          *       a quiet kingdom       *"#
        ),
        68,
        13,
    )
}

pub fn grand_cosmos(frame: usize) -> String {
    const WIDTH: usize = 68;
    let mut canvas = vec![vec![' '; WIDTH]; 13];
    for (index, x) in [3, 12, 23, 37, 49, 61, 66].into_iter().enumerate() {
        let y = (index * 3 + frame / 3) % 11;
        let star = if (frame + index).is_multiple_of(3) {
            "*"
        } else {
            "."
        };
        blit(&mut canvas, &[star], x, y);
    }
    let planet = [
        r"                         _..._",
        r"                    _.-'     `-._",
        r"              _.---'   .-·-.   `---._",
        r"        _____/_________/_____\_________\_____",
        r"             `-.       \     /       .-'",
        r"                `-._    `-.-'    _.-'",
        r"                    `--._____.--'",
    ];
    blit(&mut canvas, &planet, 0, 3);
    let ship_x = (frame * 2 % (WIDTH + 16)) as isize - 12;
    blit(&mut canvas, &["<=={o}==>"], ship_x, 1 + frame / 8 % 2);
    canvas_to_string(canvas)
}

fn blit(canvas: &mut [Vec<char>], sprite: &[&str], x: isize, y: usize) {
    for (dy, line) in sprite.iter().enumerate() {
        let Some(row) = canvas.get_mut(y + dy) else {
            continue;
        };
        for (dx, character) in line.chars().enumerate() {
            let target_x = x + dx as isize;
            if target_x >= 0 && target_x < row.len() as isize && character != ' ' {
                row[target_x as usize] = character;
            }
        }
    }
}

fn normalize(frame: &str, width: usize, height: usize) -> String {
    let mut lines: Vec<String> = frame
        .lines()
        .take(height)
        .map(|line| format!("{line:<width$}"))
        .collect();
    lines.resize(height, " ".repeat(width));
    lines.join("\n")
}

fn canvas_to_string(canvas: Vec<Vec<char>>) -> String {
    canvas
        .into_iter()
        .map(|row| row.into_iter().collect::<String>().trim_end().to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use unicode_width::UnicodeWidthStr;

    #[test]
    fn fixed_scenes_do_not_jump_in_size() {
        for frames in [
            (0..TOMATO.len()).map(tomato).collect::<Vec<_>>(),
            (0..COFFEE.len()).map(coffee).collect::<Vec<_>>(),
            (0..CAT.len()).map(cat).collect::<Vec<_>>(),
            (0..PLANT.len()).map(plant).collect::<Vec<_>>(),
            (0..HOURGLASS.len()).map(hourglass).collect::<Vec<_>>(),
            (0..FIREPLACE.len()).map(fireplace).collect::<Vec<_>>(),
        ] {
            let dimensions: Vec<_> = frames
                .iter()
                .map(|frame| {
                    (
                        frame.lines().count(),
                        frame.lines().map(UnicodeWidthStr::width).max().unwrap_or(0),
                    )
                })
                .collect();
            assert!(dimensions.windows(2).all(|pair| pair[0] == pair[1]));
        }
    }

    #[test]
    fn sky_is_bounded() {
        for frame in 0..80 {
            let art = sky(frame);
            assert_eq!(art.lines().count(), 7);
            assert!(art.lines().all(|line| line.len() <= 34));
        }
    }

    #[test]
    fn procedural_scenes_are_bounded() {
        for frame in 0..100 {
            for art in [aquarium(frame), rocket(frame)] {
                assert!(art.lines().count() <= 8);
                assert!(art.lines().all(|line| line.len() <= 34));
            }
        }
    }

    #[test]
    fn grand_scenes_stay_within_their_canvas() {
        for frame in 0..100 {
            for art in [grand_cat(frame), grand_castle(frame), grand_cosmos(frame)] {
                assert!(art.lines().count() <= 13);
                assert!(art.lines().all(|line| UnicodeWidthStr::width(line) <= 68));
            }
        }
    }
}
