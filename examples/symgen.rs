//! Scrambles of positions with a symmetry (random words in the moves and
//! move pairs fixed by a symmetry `t`) or an antisymmetry (`A · t(A)⁻¹`, `t`
//! an involution), for testing and timing symmetric start (`symstart`):
//!
//! ```text
//! symgen <word length> <count> <seed> [sym|anti|mix]
//! ```
use cubeopt::cubie::{conjugate, format_moves, inverse_move, symmetries, CubieCube, Rng};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let length: usize = args[1].parse().unwrap();
    let count: usize = args[2].parse().unwrap();
    let mut rng = Rng(args[3].parse().unwrap());
    let kind = args.get(4).map_or("mix", String::as_str);
    let syms = symmetries();
    let mut written = 0;
    while written < count {
        let sym = &syms[1 + rng.below_usize(47)];
        let fixed = |cube: &CubieCube| conjugate(cube, sym) == *cube;
        if kind == "anti" || kind == "mix" && written % 3 == 2 {
            // Antisymmetric: A · t(A)⁻¹ with t an involution.
            let probe = CubieCube::SOLVED.apply_moves(&[0, 7, 14]);
            if conjugate(&conjugate(&probe, sym), sym) != probe {
                continue;
            }
            let half: Vec<u8> = (0..length / 2).map(|_| rng.below_u8(18)).collect();
            // t(A)⁻¹ as moves: the inverses of t's move images, reversed.
            let image_inverse = half
                .iter()
                .rev()
                .map(|&mv| inverse_move(sym.move_map[usize::from(mv)]));
            let moves: Vec<u8> = half.iter().copied().chain(image_inverse).collect();
            let cube = CubieCube::SOLVED.apply_moves(&moves);
            assert_eq!(conjugate(&cube, sym), cube.inverse());
            println!("{}", format_moves(&moves));
            written += 1;
            continue;
        }
        // Words fixed by `sym`: single moves and pairs of moves on
        // different faces.
        let mut words: Vec<Vec<u8>> = (0..18u8)
            .filter(|&mv| fixed(&CubieCube::SOLVED.apply_moves(&[mv])))
            .map(|mv| vec![mv])
            .collect();
        for first in 0..18u8 {
            for second in 0..18u8 {
                if first / 3 != second / 3
                    && fixed(&CubieCube::SOLVED.apply_moves(&[first, second]))
                {
                    words.push(vec![first, second]);
                }
            }
        }
        let faces = (0..6u8)
            .filter(|face| {
                words
                    .iter()
                    .any(|word| word.iter().any(|mv| mv / 3 == *face))
            })
            .count();
        if words.len() < 6 || faces < 4 {
            continue;
        }
        let mut moves = Vec::new();
        while moves.len() < length {
            moves.extend(&words[rng.below_usize(words.len())]);
        }
        assert!(fixed(&CubieCube::SOLVED.apply_moves(&moves)));
        println!("{}", format_moves(&moves));
        written += 1;
    }
}
