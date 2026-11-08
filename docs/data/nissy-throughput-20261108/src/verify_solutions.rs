//! Replay complete JSONL benchmark batches with egg's cube model; no table needed.
use egg_solver::cubie::{parse_moves, CubieCube};
use std::{collections::HashSet, error::Error, path::Path};

fn cubes(path: &Path) -> Result<Vec<CubieCube>, Box<dyn Error>> {
    std::fs::read_to_string(path)?
        .lines()
        .map(|line| {
            parse_moves(line)
                .and_then(|moves| CubieCube::SOLVED.apply_moves(&moves))
                .ok_or_else(|| "invalid input cube".into())
        })
        .collect()
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let input = args
        .next()
        .ok_or("expected INPUT followed by JSONL batches")?;
    let states = cubes(Path::new(&input))?;
    let mut batches = 0;
    let mut verified = 0;
    for path in args {
        let mut seen = HashSet::new();
        for line in std::fs::read_to_string(&path)?.lines() {
            let value: serde_json::Value = serde_json::from_str(line)?;
            if value.get("type").and_then(serde_json::Value::as_str) != Some("solved") {
                continue;
            }
            let index = usize::try_from(
                value
                    .get("index")
                    .and_then(serde_json::Value::as_u64)
                    .ok_or("missing index")?,
            )?;
            let moves = value
                .get("moves")
                .and_then(serde_json::Value::as_str)
                .and_then(parse_moves)
                .ok_or("invalid solution moves")?;
            if !seen.insert(index)
                || states.get(index).and_then(|cube| cube.apply_moves(&moves))
                    != Some(CubieCube::SOLVED)
            {
                return Err(format!("invalid or duplicate solution: {path}, cube {index}").into());
            }
            verified += 1;
        }
        if seen.len() != states.len() {
            return Err(format!("incomplete batch: {path}").into());
        }
        batches += 1;
    }
    if batches == 0 {
        return Err("no batches supplied".into());
    }
    println!("{{\"status\":\"PASS\",\"batches\":{batches},\"verified\":{verified}}}");
    Ok(())
}
