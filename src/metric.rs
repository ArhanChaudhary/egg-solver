//! Move metrics: how the length of a solution is counted.
//!
//! * HTM (half-turn metric, the default): any turn of a face is one move.
//! * QTM (quarter-turn metric): a quarter turn is one move, a half turn two.
//!
//! The search is the same in both: the 18 face turns, in canonical order
//! (`solver::ALLOWED_AFTER`), with a half turn charged two moves of the
//! target in QTM. A QTM-optimal solution has a canonical form of the same
//! QTM length: `X X` is `X2`, and three turns of one axis (`U D U`) are two
//! (`U2 D`). God's number is 20 in HTM and 26 in QTM.
//!
//! Every quarter turn is an odd permutation of the corners (a 4-cycle), and
//! a half turn an even one, so the QTM length of every solution of a cube has
//! the parity of its corner permutation: QTM iterations step by two.
//!
//! Table values of one metric are lower bounds for the other only one way:
//! an HTM distance is at most the QTM distance (a half turn is one HTM move
//! but two QTM moves), so HTM tables can bound a QTM search, and QTM tables
//! cannot bound an HTM search.

/// A move metric (see the module docs). (In JSON: `"htm"` or `"qtm"`.)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "cli",
    derive(serde::Serialize),
    serde(rename_all = "lowercase")
)]
pub enum Metric {
    /// Half-turn metric: every face turn is one move.
    #[default]
    Htm,
    /// Quarter-turn metric: a half turn is two moves.
    Qtm,
}

impl Metric {
    /// Both metrics.
    pub const ALL: [Metric; 2] = [Metric::Htm, Metric::Qtm];

    /// The metric's name: `htm` or `qtm`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Metric::Htm => "htm",
            Metric::Qtm => "qtm",
        }
    }

    /// The metric named `name` (`htm` or `qtm`, any case).
    #[must_use]
    pub fn parse(name: &str) -> Option<Metric> {
        Metric::ALL
            .into_iter()
            .find(|metric| metric.name().eq_ignore_ascii_case(name.trim()))
    }

    /// The suffix of a length in this metric, as in "18f" and "22q".
    #[must_use]
    pub const fn suffix(self) -> char {
        match self {
            Metric::Htm => 'f',
            Metric::Qtm => 'q',
        }
    }

    /// Whether this is QTM (for the searches' `const QTM: bool` parameter).
    #[must_use]
    pub const fn is_qtm(self) -> bool {
        matches!(self, Metric::Qtm)
    }

    /// The cost of move `mv` (numbered as `cubie::Move`).
    #[must_use]
    #[inline(always)]
    pub const fn move_cost(self, mv: u8) -> u8 {
        move_cost(self.is_qtm(), mv)
    }

    /// The length of a move sequence in this metric.
    #[must_use]
    pub fn length(self, moves: &[u8]) -> usize {
        moves
            .iter()
            .map(|&mv| usize::from(self.move_cost(mv)))
            .sum()
    }

    /// God's number: the largest distance of any position (20 in HTM, 26
    /// in QTM), the last target a search tries.
    #[must_use]
    pub const fn max_target(self) -> u8 {
        match self {
            Metric::Htm => crate::solver::MAX_TARGET,
            Metric::Qtm => crate::solver::MAX_TARGET_QTM,
        }
    }

    /// Whether lower bounds for distances in metric `of` are also lower
    /// bounds in this metric: a table of `of` can bound a search in `self`.
    #[must_use]
    pub const fn admits(self, of: Metric) -> bool {
        matches!((self, of), (Metric::Qtm, _) | (Metric::Htm, Metric::Htm))
    }
}

impl std::fmt::Display for Metric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl std::str::FromStr for Metric {
    type Err = String;

    fn from_str(text: &str) -> Result<Metric, String> {
        Metric::parse(text).ok_or_else(|| format!("unknown metric {text:?}: expected htm or qtm"))
    }
}

/// The cost of move `mv` in QTM if `qtm`, else in HTM (the searches'
/// `const QTM: bool` form): 2 for a half turn in QTM, else 1.
#[must_use]
#[inline(always)]
pub const fn move_cost(qtm: bool, mv: u8) -> u8 {
    1 + (qtm && mv % 3 == 1) as u8
}

/// A move sequence in canonical form, with the same cube effect: the turns
/// of each run of consecutive moves on one axis are merged per face (`R R`
/// is `R2`, `U D U` is `U2 D`, `R R'` nothing) and ordered as the search
/// orders them (U before D, R before L, F before B). No metric's length
/// grows. A QTM search can return `X X` where its two sides meet (the same
/// QTM length as `X2`); this gives the usual form.
#[must_use]
pub fn simplify(moves: &[u8]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::with_capacity(moves.len());
    for &mv in moves {
        let face = mv / 3;
        let n = out.len();
        // The earlier move of the same face, if `mv` can merge with it: the
        // last one, or the one before it if the last turns the opposite
        // face (they commute). (Kept as an invariant: no two moves of a face
        // and no three of an axis follow each other in `out`.)
        let same = if n >= 1 && out[n - 1] / 3 == face {
            Some(n - 1)
        } else if n >= 2 && out[n - 1] / 6 == mv / 6 && out[n - 2] / 3 == face {
            Some(n - 2)
        } else {
            None
        };
        if let Some(i) = same {
            match (out[i] % 3 + mv % 3 + 2) % 4 {
                0 => {
                    out.remove(i);
                }
                quarter_turns => out[i] = 3 * face + quarter_turns - 1,
            }
        } else {
            out.push(mv);
            // (U before D, R before L, F before B.)
            if n >= 1 && out[n - 1] / 6 == mv / 6 && out[n - 1] > mv {
                out.swap(n - 1, n);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cubie::{parse_moves, CubieCube, Rng};

    fn moves(text: &str) -> Vec<u8> {
        parse_moves(text).unwrap()
    }

    #[test]
    fn costs_and_lengths() {
        assert_eq!(Metric::Htm.length(&moves("R U2 F' B2")), 4);
        assert_eq!(Metric::Qtm.length(&moves("R U2 F' B2")), 6);
        assert_eq!(Metric::parse("QTM"), Some(Metric::Qtm));
        assert_eq!(Metric::parse("htm"), Some(Metric::Htm));
        assert_eq!(Metric::parse("stm"), None);
        assert_eq!(Metric::default(), Metric::Htm);
        assert!(Metric::Qtm.admits(Metric::Htm) && Metric::Qtm.admits(Metric::Qtm));
        assert!(Metric::Htm.admits(Metric::Htm) && !Metric::Htm.admits(Metric::Qtm));
        for mv in 0..18u8 {
            assert_eq!(move_cost(false, mv), 1);
            assert_eq!(move_cost(true, mv), if mv % 3 == 1 { 2 } else { 1 });
        }
    }

    #[test]
    fn simplify_merges_axis_runs() {
        let cases = [
            ("R R", "R2"),
            ("U D U", "U2 D"),
            ("D U", "U D"),
            ("R R'", ""),
            ("F R R' F", "F2"),
            ("U R L R' L' U'", ""),
            ("U2 D U2", "D"),
            ("R U F", "R U F"),
            ("L R", "R L"),
            ("B F B", "F B2"),
        ];
        for (input, want) in cases {
            assert_eq!(simplify(&moves(input)), moves(want), "{input}");
        }
        // Random sequences: the same cube, canonical, no longer in either
        // metric.
        let mut rng = Rng(3);
        for _ in 0..2000 {
            let n = rng.below_usize(12);
            let seq: Vec<u8> = (0..n).map(|_| rng.below_u8(18)).collect();
            let out = simplify(&seq);
            let cube = |m: &[u8]| CubieCube::SOLVED.apply_moves(m).unwrap();
            assert_eq!(cube(&seq), cube(&out), "{seq:?} -> {out:?}");
            for w in out.windows(2) {
                let (a, b) = (w[0] / 3, w[1] / 3);
                assert!(a != b && !(a / 2 == b / 2 && a > b), "{seq:?} -> {out:?}");
            }
            for w in out.windows(3) {
                assert!(!(w[0] / 6 == w[1] / 6 && w[1] / 6 == w[2] / 6), "{out:?}");
            }
            for metric in Metric::ALL {
                assert!(metric.length(&out) <= metric.length(&seq));
            }
        }
    }
}
